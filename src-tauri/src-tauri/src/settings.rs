//! 用户设置管理
//!
//! 支持从数据库加载和保存用户配置
//! API Key 使用系统密钥链安全存储

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use crate::db::operations::{get_all_settings, get_setting, set_setting};

const KEYRING_SERVICE: &str = "com.zenflow.app";
const KEYRING_USERNAME_SILICONFLOW: &str = "siliconflow_api_key";
const KEYRING_USERNAME_EMBEDDING: &str = "embedding_api_key";
const KEYRING_USERNAME_TRANSLATION: &str = "translation_api_key";

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// arXiv 分类列表
    pub arxiv_categories: Vec<String>,
    /// SiliconFlow API Key (存储在系统密钥链)
    #[serde(skip)]
    pub siliconflow_api_key: String,
    /// 嵌入向量 API Base URL
    pub embedding_api_base_url: String,
    /// 嵌入向量 API Key (存储在系统密钥链)
    #[serde(skip)]
    pub embedding_api_key: String,
    /// 嵌入向量模型
    pub embedding_model: String,
    /// 翻译 API Base URL
    pub translation_api_base_url: String,
    /// 翻译 API Key (存储在系统密钥链)
    #[serde(skip)]
    pub translation_api_key: String,
    /// 翻译模型
    pub translation_model: String,
    /// 正向聚类数量
    pub pos_clusters: usize,
    /// 负向聚类数量
    pub neg_clusters: usize,
    /// 每天展示论文数量
    pub daily_papers: usize,
    /// 负向惩罚系数 alpha
    pub negative_alpha: f32,
    /// 多样性比例 (0-1)，即随机推荐占比
    pub diversity_ratio: f32,
}

/// 从系统密钥链获取 API Key
fn get_api_key_from_keyring(username: &str) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, username).ok()?;
    match entry.get_password() {
        Ok(key) if !key.is_empty() => Some(key),
        _ => None,
    }
}

/// 保存 API Key 到系统密钥链
pub fn save_api_key_to_keyring(username: &str, api_key: &str) -> anyhow::Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, username)?;
    entry.set_password(api_key)?;
    Ok(())
}

/// 删除系统密钥链中的 API Key
#[allow(dead_code)]
fn delete_api_key_from_keyring(username: &str) -> anyhow::Result<()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, username)?;
    entry.delete_credential()?;
    Ok(())
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            arxiv_categories: vec!["cs.AI".to_string(), "cs.LG".to_string(), "cs.CL".to_string()],
            siliconflow_api_key: String::new(),
            embedding_api_base_url: "https://api.siliconflow.cn/v1".to_string(),
            embedding_api_key: String::new(),
            embedding_model: "BAAI/bge-m3".to_string(),
            translation_api_base_url: "https://api.openai.com/v1".to_string(),
            translation_api_key: String::new(),
            translation_model: "gpt-3.5-turbo".to_string(),
            pos_clusters: 5,
            neg_clusters: 3,
            daily_papers: 20,
            negative_alpha: 1.5,
            diversity_ratio: 0.3,
        }
    }
}

impl AppSettings {
    /// 从数据库加载设置
    pub fn load() -> anyhow::Result<Self> {
        let settings_map = get_all_settings()?;
        
        let mut settings = Self::default();
        
        if let Some(categories) = settings_map.get("arxiv_categories") {
            settings.arxiv_categories = categories
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        
        // 从系统密钥链获取 API Key
        settings.siliconflow_api_key = get_api_key_from_keyring(KEYRING_USERNAME_SILICONFLOW).unwrap_or_default();
        settings.embedding_api_key = get_api_key_from_keyring(KEYRING_USERNAME_EMBEDDING).unwrap_or_default();
        settings.translation_api_key = get_api_key_from_keyring(KEYRING_USERNAME_TRANSLATION).unwrap_or_default();
        
        // 嵌入向量 API 配置
        if let Some(v) = settings_map.get("embedding_api_base_url") {
            settings.embedding_api_base_url = v.clone();
        }
        if let Some(v) = settings_map.get("embedding_model") {
            settings.embedding_model = v.clone();
        }
        
        // 翻译 API 配置
        if let Some(v) = settings_map.get("translation_api_base_url") {
            settings.translation_api_base_url = v.clone();
        }
        if let Some(v) = settings_map.get("translation_model") {
            settings.translation_model = v.clone();
        }
        
        if let Some(v) = settings_map.get("pos_clusters") {
            if let Ok(n) = v.parse::<usize>() {
                settings.pos_clusters = n;
            }
        }
        
        if let Some(v) = settings_map.get("neg_clusters") {
            if let Ok(n) = v.parse::<usize>() {
                settings.neg_clusters = n;
            }
        }
        
        if let Some(v) = settings_map.get("daily_papers") {
            if let Ok(n) = v.parse::<usize>() {
                settings.daily_papers = n;
            }
        }
        
        if let Some(v) = settings_map.get("negative_alpha") {
            if let Ok(f) = v.parse::<f32>() {
                settings.negative_alpha = f;
            }
        }
        
        if let Some(v) = settings_map.get("diversity_ratio") {
            if let Ok(f) = v.parse::<f32>() {
                settings.diversity_ratio = f.clamp(0.0, 1.0);
            }
        }
        
        Ok(settings)
    }
    
    /// 保存到数据库
    pub fn save(&self) -> anyhow::Result<()> {
        set_setting("arxiv_categories", &self.arxiv_categories.join(","))?;
        // API Key 保存到系统密钥链
        if !self.siliconflow_api_key.is_empty() {
            save_api_key_to_keyring(KEYRING_USERNAME_SILICONFLOW, &self.siliconflow_api_key)?;
        }
        if !self.embedding_api_key.is_empty() {
            save_api_key_to_keyring(KEYRING_USERNAME_EMBEDDING, &self.embedding_api_key)?;
        }
        if !self.translation_api_key.is_empty() {
            save_api_key_to_keyring(KEYRING_USERNAME_TRANSLATION, &self.translation_api_key)?;
        }
        // 嵌入向量 API 配置
        set_setting("embedding_api_base_url", &self.embedding_api_base_url)?;
        set_setting("embedding_model", &self.embedding_model)?;
        // 翻译 API 配置
        set_setting("translation_api_base_url", &self.translation_api_base_url)?;
        set_setting("translation_model", &self.translation_model)?;
        // 推荐参数
        set_setting("pos_clusters", &self.pos_clusters.to_string())?;
        set_setting("neg_clusters", &self.neg_clusters.to_string())?;
        set_setting("daily_papers", &self.daily_papers.to_string())?;
        set_setting("negative_alpha", &self.negative_alpha.to_string())?;
        set_setting("diversity_ratio", &self.diversity_ratio.to_string())?;
        Ok(())
    }
    
    /// 检查是否已完成初始化
    pub fn is_initialized() -> anyhow::Result<bool> {
        match get_setting("initialized")? {
            Some(v) => Ok(v == "true"),
            None => Ok(false),
        }
    }
    
    /// 标记为已初始化
    pub fn mark_initialized() -> anyhow::Result<()> {
        set_setting("initialized", "true")?;
        Ok(())
    }
    
    /// 重置初始化状态（用于重新配置）
    pub fn reset_initialized() -> anyhow::Result<()> {
        set_setting("initialized", "false")?;
        Ok(())
    }
    
    /// 获取 RSS 订阅列表
    pub fn get_rss_feeds(&self) -> Vec<String> {
        self.arxiv_categories
            .iter()
            .map(|cat| format!("https://rss.arxiv.org/rss/{}", cat))
            .collect()
    }
}

// 全局设置缓存
use once_cell::sync::Lazy;

static SETTINGS_CACHE: Lazy<Mutex<Option<AppSettings>>> = Lazy::new(|| Mutex::new(None));

/// 获取当前设置（带缓存）
pub fn get_settings() -> anyhow::Result<AppSettings> {
    let mut cache = SETTINGS_CACHE.lock().unwrap();
    if cache.is_none() {
        *cache = Some(AppSettings::load()?);
    }
    Ok(cache.clone().unwrap())
}

/// 刷新设置缓存
pub fn refresh_settings() -> anyhow::Result<AppSettings> {
    let mut cache = SETTINGS_CACHE.lock().unwrap();
    let settings = AppSettings::load()?;
    *cache = Some(settings.clone());
    Ok(settings)
}

/// 更新设置
pub fn update_settings(settings: &AppSettings) -> anyhow::Result<()> {
    settings.save()?;
    let mut cache = SETTINGS_CACHE.lock().unwrap();
    *cache = Some(settings.clone());
    Ok(())
}

/// 获取 SiliconFlow API Key（优先从设置获取）
pub fn get_api_key() -> anyhow::Result<Option<String>> {
    let settings = get_settings()?;
    if !settings.siliconflow_api_key.is_empty() {
        return Ok(Some(settings.siliconflow_api_key));
    }
    // 回退到环境变量
    Ok(std::env::var("SILICONFLOW_API_KEY").ok())
}