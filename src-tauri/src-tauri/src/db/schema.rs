//! 数据库 Schema 定义

pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS articles (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    link TEXT NOT NULL,
    abstract TEXT,
    source TEXT NOT NULL,
    vector BLOB,
    status INTEGER DEFAULT 0,
    score REAL DEFAULT 0.0,
    translated_title TEXT,
    translated_abstract TEXT,
    author TEXT,
    category TEXT,
    hf_upvotes INTEGER,
    ax_upvotes INTEGER,
    ax_downvotes INTEGER,
    votes_updated_at DATETIME,
    comment TEXT,
    timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- 用户设置表
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_articles_status ON articles(status);
CREATE INDEX IF NOT EXISTS idx_articles_score ON articles(score DESC);
CREATE INDEX IF NOT EXISTS idx_articles_timestamp ON articles(timestamp);
"#;

/// 迁移 SQL：清理旧表 + 添加新列
pub const MIGRATION_SQL: &str = r#"
DROP TABLE IF EXISTS clusters;
DROP INDEX IF EXISTS idx_clusters_type;
"#;

/// 添加 comment 列（已有数据库兼容）
pub const MIGRATION_ADD_COMMENT: &str = "ALTER TABLE articles ADD COLUMN comment TEXT;";
