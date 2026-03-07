//! 数据库连接池管理

use anyhow::Result;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::OnceLock;

use super::schema::SCHEMA_SQL;
use crate::config::db_path;

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = PooledConnection<SqliteConnectionManager>;

static DB_POOL: OnceLock<DbPool> = OnceLock::new();

/// 初始化数据库连接池
pub fn init_db() -> Result<DbPool> {
    let db_path = db_path();
    
    // 确保目录存在
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    let manager = SqliteConnectionManager::file(&db_path);
    let pool = Pool::builder()
        .max_size(5)
        .build(manager)?;
    
    // 初始化 schema
    let conn = pool.get()?;
    conn.execute_batch(SCHEMA_SQL)?;
    
    DB_POOL.set(pool.clone()).map_err(|_| anyhow::anyhow!("Database pool already initialized"))?;
    
    tracing::info!("📦 数据库初始化完成: {}", db_path);
    Ok(pool)
}

/// 获取数据库连接
pub fn get_db() -> Result<DbConnection> {
    DB_POOL
        .get()
        .ok_or_else(|| anyhow::anyhow!("Database not initialized"))?
        .get()
        .map_err(Into::into)
}