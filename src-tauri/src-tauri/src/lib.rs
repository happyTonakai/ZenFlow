//! ZenFlow - AI Paper & News Recommendation Agent

pub mod config;
pub mod db;
pub mod feed;
pub mod llm;
pub mod algorithm;
pub mod commands;
pub mod settings;
pub mod extractor;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    // 初始化数据库
    match db::init_db() {
        Ok(_) => tracing::info!("数据库初始化成功"),
        Err(e) => {
            tracing::error!("数据库初始化失败: {}", e);
            std::process::exit(1);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .invoke_handler(tauri::generate_handler![
            // 文章操作
            commands::fetch_articles,
            commands::get_articles,
            commands::get_recommended_articles,
            commands::update_status,
            commands::add_comment,
            commands::mark_all_read,
            // 推荐系统
            commands::refresh_recommendations,
            commands::update_preferences,
            commands::is_initialized,
            commands::get_stats,
            commands::clean_old_articles,
            // 初始化和设置
            commands::save_settings,
            commands::get_settings,
            commands::needs_initialization,
            commands::reset_initialization,
            commands::fetch_favorite_papers,
            commands::initialize_app,
            commands::get_arxiv_categories,
            commands::translate_text,
            commands::translate_batch,
            commands::request_keychain_access,
            // 论文提取
            commands::extract_paper,
            commands::extract_paper_to_clipboard,
            commands::extract_paper_latex,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
