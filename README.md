# Deep Archive

Project Deep Archive is a production-ready, multi-threaded media preservation pipeline built in Rust. It ingests media files, performs AI-based content analysis (NSFW detection and tagging), stores metadata in a SQLite database, and creates archival-ready ISO images.

## Quick Start

To prepare the runtime environment, run the setup script:

```bash
./setup.sh
```

This script will:
1. Create necessary directories (`models`, `data`, `iso`).
2. Check for system dependencies (`ffmpeg`, `xorriso`).
3. Download the required ONNX models.

## Usage

Once the environment is set up, you can run the pipeline with the following command:

```bash
cargo run --release -- --input-dir ./media --db-path ./data/archive_index.db --output-iso iso/archive.iso
```

### Arguments

* `--input-dir`: Path to the directory containing media files to ingest.
* `--db-path`: Path where the SQLite database index will be stored.
* `--output-iso`: (Optional) Path to create the archival ISO file.
