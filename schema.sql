CREATE TABLE IF NOT EXISTS articles (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    link TEXT NOT NULL,
    abstract TEXT,
    source TEXT NOT NULL,
    vector BLOB,
    status INTEGER DEFAULT 0,
    score REAL DEFAULT 0.0,
    translated_abstract TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS clusters (
    type TEXT PRIMARY KEY,
    centroid BLOB
);

CREATE INDEX IF NOT EXISTS idx_articles_status ON articles(status);
CREATE INDEX IF NOT EXISTS idx_articles_score ON articles(score DESC);
CREATE INDEX IF NOT EXISTS idx_articles_timestamp ON articles(timestamp);
