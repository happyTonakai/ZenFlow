//! 数据库连接池管理

use anyhow::Result;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::OnceLock;

use super::schema::{SCHEMA_SQL, MIGRATION_SQL, MIGRATION_ADD_COMMENT};
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

    // 运行迁移（清理旧的 clusters 表）
    if let Err(e) = conn.execute_batch(MIGRATION_SQL) {
        tracing::warn!("数据库迁移警告（可忽略）: {}", e);
    }

    // 添加 comment 列（已有列时会报错，可忽略）
    if let Err(_) = conn.execute(MIGRATION_ADD_COMMENT, []) {
        // 列已存在，忽略
    }

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

/// 创建内存数据库连接池（用于测试）
#[cfg(test)]
pub fn create_test_pool() -> Result<DbPool> {
    let manager = SqliteConnectionManager::memory();
    let pool = Pool::builder().max_size(1).build(manager)?;
    let conn = pool.get()?;
    conn.execute_batch(SCHEMA_SQL)?;
    let _ = conn.execute_batch(MIGRATION_SQL);
    let _ = conn.execute(MIGRATION_ADD_COMMENT, []);
    Ok(pool)
}

/// 设置全局测试数据库池（仅首次调用生效）
#[cfg(test)]
pub fn init_test_db() -> Result<DbPool> {
    let pool = create_test_pool()?;
    let _ = DB_POOL.set(pool.clone());
    Ok(pool)
}
