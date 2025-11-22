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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input_dir: PathBuf,

    #[arg(short, long)]
    db_path: String,

    #[arg(short, long)]
    output_iso: Option<PathBuf>,
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

    // Initialize ML Engine (Placeholder paths - would normally come from config)
    // We wrap it in Arc to share across threads.
    // Note: This requires models to exist at these paths. For the purpose of this exercise
    // where we don't have the models, we will allow the pipeline to proceed even if inference fails,
    // or wrap the engine in an Option if initialization fails.
    let engine = match InferenceEngine::new("models/nsfw.onnx", "models/tagger.onnx") {
        Ok(e) => Some(Arc::new(e)),
        Err(e) => {
            error!("Failed to initialize AI Engine (check model paths): {}", e);
            None
        }
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
    // Drop the original tx so receiver closes when all hashers are done
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
                // Detect Mimetype
                let media_type = match mimetype::detect_mimetype(&job.path) {
                    Ok(m) => m,
                    Err(e) => {
                        error!("Mimetype detection failed for {:?}: {}", job.path, e);
                        "application/octet-stream".to_string()
                    }
                };

                let mut nsfw_score = None;
                let mut tags = Vec::new();

                // Only process video/image types that ffmpeg can handle
                if media_type.starts_with("video/") || media_type.starts_with("image/") {
                     match ffmpeg::extract_frames(&job.path) {
                        Ok(raw_bytes) => {
                            // Convert raw bytes (RGB24 224x224) to DynamicImage
                            // ffmpeg.rs ensures output is 224x224 RGB24
                            if let Some(img_buffer) = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(224, 224, raw_bytes) {
                                let dynamic_image = image::DynamicImage::ImageRgb8(img_buffer);

                                if let Some(ref _eng) = engine {
                                    // NSFW Check
                                    match pipeline::normalize_for_nsfw(&dynamic_image) {
                                        Ok(_input) => {
                                            // Real inference would go here:
                                            // let _res = eng.nsfw_session().run(ort::inputs![input]...);
                                            // For now, simulate score
                                            nsfw_score = Some(0.01);
                                        }
                                        Err(e) => error!("NSFW normalization failed: {}", e),
                                    }

                                    // Tagger Check
                                    // Note: Tagger might need 448x448, but we only extracted 224x224 from ffmpeg.
                                    // In a real scenario, we might need two extractions or resize here.
                                    // For this exercise, we'll skip or just reuse the image (it will be resized in normalize).
                                    match pipeline::normalize_for_tagger(&dynamic_image) {
                                         Ok(_input) => {
                                            // Real inference...
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
                             // Log but don't crash, regular file or unsupported format for ffmpeg
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
                    width: Some(224), // We scaled it
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

    // Wait for all
    scanner_handle.join().unwrap();
    for h in hasher_handles { h.join().unwrap(); }
    for h in worker_handles { h.join().unwrap(); }
    db_handle.join().unwrap();

    // Archival Phase (if requested)
    if let Some(iso_path) = args.output_iso {
        info!("Creating ISO archive at {:?}", iso_path);
        if let Err(e) = crate::archive::iso_builder::create_iso(&args.input_dir, &iso_path) {
            error!("Archival failed: {}", e);
        } else {
            info!("ISO created successfully.");
        }
    }

    info!("Pipeline completed.");
    Ok(())
}
