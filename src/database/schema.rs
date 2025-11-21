pub const SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS artifacts (
        id INTEGER PRIMARY KEY,
        hash_sha256 TEXT UNIQUE NOT NULL,
        original_path TEXT NOT NULL,
        media_type TEXT NOT NULL,
        width INTEGER,
        height INTEGER
    );

    CREATE TABLE IF NOT EXISTS tags (
        id INTEGER PRIMARY KEY,
        name TEXT UNIQUE NOT NULL
    );

    CREATE TABLE IF NOT EXISTS artifact_tags (
        artifact_id INTEGER NOT NULL,
        tag_id INTEGER NOT NULL,
        FOREIGN KEY(artifact_id) REFERENCES artifacts(id),
        FOREIGN KEY(tag_id) REFERENCES tags(id),
        PRIMARY KEY(artifact_id, tag_id)
    );

    CREATE TABLE IF NOT EXISTS safety_scores (
        artifact_id INTEGER PRIMARY KEY,
        nsfw_score REAL NOT NULL,
        FOREIGN KEY(artifact_id) REFERENCES artifacts(id)
    );

    CREATE VIRTUAL TABLE IF NOT EXISTS search_index USING fts5(original_path, tags_concatenated);
";
