mod ingest;
mod media;
mod ml;
mod database;
mod archive;
mod utils;

use std::path::PathBuf;
use std::thread;
use std::sync::Arc;
use crossbeam::channel::bounded;
use anyhow::Result;
use clap::Parser;
use tracing::{info, error};
use image::{ImageBuffer, Rgb};

use crate::ingest::{scanner, hasher};
use crate::database::repo::{TransactionManager, ArtifactRecord};
use crate::ml::engine::InferenceEngine;
use crate::ml::pipeline;
use crate::media::ffmpeg;
use crate::media::mimetype;
use crate::utils::config;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input_dir: PathBuf,

    #[arg(short, long)]
    db_path: String,

    #[arg(short, long, default_value = "iso/archive.iso")]
    output_iso: PathBuf,
}

struct MediaJob {
    path: PathBuf,
    hash: String,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!("Deep Archive Pipeline Starting...");
    info!("Input: {:?}", args.input_dir);
    info!("DB: {}", args.db_path);

    // 1. Locate Models (Auto-search + .env generation)
    let model_paths = match config::get_model_paths() {
        Ok(paths) => Some(paths),
        Err(e) => {
            error!("Failed to initialize AI Engine: {}. \n\nHint: Have you run './setup.sh' to download the models?", e);
            None
        }
    };

    // 2. Initialize ML Engine
    let engine = if let Some(paths) = model_paths {
        let nsfw_str = paths.nsfw.to_string_lossy().to_string();
        let tagger_str = paths.tagger.to_string_lossy().to_string();

        match InferenceEngine::new(&nsfw_str, &tagger_str) {
            Ok(e) => Some(Arc::new(e)),
            Err(e) => {
                error!("Failed to initialize AI Engine with found paths: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Channels
    let (scan_tx, scan_rx) = bounded::<PathBuf>(1024);
    let (hash_tx, hash_rx) = bounded::<MediaJob>(1024);
    let (db_tx, db_rx) = bounded::<ArtifactRecord>(1024);

    // 1. Scanner Thread
    let input_dir = args.input_dir.clone();
    let scanner_handle = thread::spawn(move || {
        info!("Scanner started");
        if let Err(e) = scanner::scan_directory(&input_dir, scan_tx) {
            error!("Scanner failed: {}", e);
        }
        info!("Scanner finished");
    });

    // 2. Hasher Threads
    let num_hashers = 4;
    let mut hasher_handles = Vec::new();

    for i in 0..num_hashers {
        let rx = scan_rx.clone();
        let tx = hash_tx.clone();
        hasher_handles.push(thread::spawn(move || {
            info!("Hasher {} started", i);
            for path in rx {
                match hasher::calculate_hash(&path) {
                    Ok(hash) => {
                        let job = MediaJob { path, hash };
                        let _ = tx.send(job);
                    },
                    Err(e) => {
                        error!("Failed to hash {:?}: {}", path, e);
                    }
                }
            }
            info!("Hasher {} finished", i);
        }));
    }
    drop(hash_tx);

    // 3. Media/AI Worker Threads
    let num_workers = 2;
    let mut worker_handles = Vec::new();

    for i in 0..num_workers {
        let rx = hash_rx.clone();
        let tx = db_tx.clone();
        let engine = engine.clone();

        worker_handles.push(thread::spawn(move || {
            info!("Worker {} started", i);
            for job in rx {
                let media_type = match mimetype::detect_mimetype(&job.path) {
                    Ok(m) => m,
                    Err(e) => {
                        error!("Mimetype detection failed for {:?}: {}", job.path, e);
                        "application/octet-stream".to_string()
                    }
                };

                let mut nsfw_score = None;
                let mut tags = Vec::new();

                if media_type.starts_with("video/") || media_type.starts_with("image/") {
                     match ffmpeg::extract_frames(&job.path) {
                        Ok(raw_bytes) => {
                            if let Some(img_buffer) = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(224, 224, raw_bytes) {
                                let dynamic_image = image::DynamicImage::ImageRgb8(img_buffer);

                                if let Some(ref _eng) = engine {
                                    match pipeline::normalize_for_nsfw(&dynamic_image) {
                                        Ok(_input) => {
                                            // Placeholder for real inference
                                            nsfw_score = Some(0.01);
                                        }
                                        Err(e) => error!("NSFW normalization failed: {}", e),
                                    }

                                    match pipeline::normalize_for_tagger(&dynamic_image) {
                                         Ok(_input) => {
                                            // Placeholder for real inference
                                            tags.push("simulated_tag".to_string());
                                         }
                                         Err(e) => error!("Tagger normalization failed: {}", e),
                                    }
                                }
                            } else {
                                error!("Failed to create ImageBuffer from raw bytes for {:?}", job.path);
                            }
                        }
                        Err(e) => {
                             if !media_type.starts_with("text") {
                                 error!("Frame extraction failed for {:?}: {}", job.path, e);
                             }
                        }
                     }
                }

                let record = ArtifactRecord {
                    hash_sha256: job.hash,
                    original_path: job.path.to_string_lossy().to_string(),
                    media_type,
                    width: Some(224),
                    height: Some(224),
                    tags,
                    nsfw_score,
                };

                let _ = tx.send(record);
            }
            info!("Worker {} finished", i);
        }));
    }
    drop(db_tx);

    // 4. DB Writer Thread
    let db_path = args.db_path.clone();
    let db_handle = thread::spawn(move || {
        info!("DB Writer started");
        let mut tm = match TransactionManager::new(&db_path) {
            Ok(tm) => tm,
            Err(e) => {
                error!("Failed to init DB: {}", e);
                return;
            }
        };

        for record in db_rx {
            if let Err(e) = tm.add(record) {
                error!("Failed to add record to DB: {}", e);
            }
        }

        if let Err(e) = tm.flush() {
             error!("Failed to flush remaining records: {}", e);
        }
        info!("DB Writer finished");
    });

    scanner_handle.join().unwrap();
    for h in hasher_handles { h.join().unwrap(); }
    for h in worker_handles { h.join().unwrap(); }
    db_handle.join().unwrap();

    info!("Creating ISO archive at {:?}", args.output_iso);
    if let Err(e) = crate::archive::iso_builder::create_iso(&args.input_dir, &args.output_iso) {
        error!("Archival failed: {}", e);
    } else {
        info!("ISO created successfully.");
    }

    info!("Pipeline completed.");
    Ok(())
}
