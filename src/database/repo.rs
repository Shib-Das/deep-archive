use rusqlite::{Connection, params};
use anyhow::{Result, Context};
use crate::database::schema::SCHEMA;

#[derive(Debug, Clone)]
pub struct ArtifactRecord {
    pub hash_sha256: String,
    pub original_path: String,
    pub media_type: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub tags: Vec<String>,
    pub nsfw_score: Option<f32>,
}

pub struct TransactionManager {
    conn: Connection,
    buffer: Vec<ArtifactRecord>,
    buffer_limit: usize,
}

impl TransactionManager {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path).context("Failed to open database")?;
        conn.execute_batch(SCHEMA).context("Failed to initialize schema")?;
        Ok(Self {
            conn,
            buffer: Vec::new(),
            buffer_limit: 1000,
        })
    }

    pub fn add(&mut self, record: ArtifactRecord) -> Result<()> {
        self.buffer.push(record);
        if self.buffer.len() >= self.buffer_limit {
            self.flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let mut tx = self.conn.transaction().context("Failed to begin transaction")?;

        {
            // We use prepared statements for efficiency.
            // Using RETURNING id is supported in modern SQLite.
            let mut stmt_artifact = tx.prepare(
                "INSERT INTO artifacts (hash_sha256, original_path, media_type, width, height)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(hash_sha256) DO UPDATE SET original_path=excluded.original_path
                 RETURNING id"
            )?;

            let mut stmt_tag = tx.prepare(
                "INSERT OR IGNORE INTO tags (name) VALUES (?1)"
            )?;

            let mut stmt_get_tag_id = tx.prepare(
                "SELECT id FROM tags WHERE name = ?1"
            )?;

            let mut stmt_artifact_tag = tx.prepare(
                "INSERT OR IGNORE INTO artifact_tags (artifact_id, tag_id) VALUES (?1, ?2)"
            )?;

            let mut stmt_score = tx.prepare(
                "INSERT OR REPLACE INTO safety_scores (artifact_id, nsfw_score) VALUES (?1, ?2)"
            )?;

            // For FTS, we might want to avoid duplicates if the file is already there,
            // but FTS doesn't have unique constraints easily.
            // We'll just insert for now, assuming the upstream pipeline handles high-level deduplication logic
            // or we accept multiple entries for now.
            let mut stmt_fts = tx.prepare(
                "INSERT INTO search_index (original_path, tags_concatenated) VALUES (?1, ?2)"
            )?;

            for record in &self.buffer {
                // Insert artifact or update
                let artifact_id: i64 = stmt_artifact.query_row(params![
                    record.hash_sha256,
                    record.original_path,
                    record.media_type,
                    record.width,
                    record.height
                ], |row| row.get(0)).context("Failed to insert/get artifact")?;

                // Handle Tags
                let mut tag_names = Vec::new();
                for tag in &record.tags {
                    stmt_tag.execute(params![tag])?;

                    let tag_id: i64 = stmt_get_tag_id.query_row(params![tag], |row| row.get(0))
                        .context("Failed to get tag id after insert")?;

                    stmt_artifact_tag.execute(params![artifact_id, tag_id])?;
                    tag_names.push(tag.as_str());
                }

                // Handle Safety Score
                if let Some(score) = record.nsfw_score {
                    stmt_score.execute(params![artifact_id, score])?;
                }

                // Handle FTS
                let tags_concat = tag_names.join(" ");
                stmt_fts.execute(params![record.original_path, tags_concat])?;
            }
        }

        tx.commit().context("Failed to commit transaction")?;
        self.buffer.clear();
        Ok(())
    }
}
