//! K-Means 聚类模块

use anyhow::Result;
use linfa::prelude::Fit;
use linfa::DatasetBase;
use linfa_clustering::KMeans;
use ndarray::{Array1, Array2};

use crate::config;
use crate::db;
use crate::settings;

/// 聚类更新结果
pub struct ClusterResult {
    pub pos_centroids: Vec<Array1<f32>>,
    pub neg_centroids: Vec<Array1<f32>>,
    pub pos_count: usize,
    pub neg_count: usize,
}

/// 更新聚类中心
/// 
/// 正向聚类: 使用 status=1 (点击) 和 status=2 (点赞)
/// - 点赞权重 = WEIGHT_LIKED (默认 2.0)
/// - 点击权重 = WEIGHT_CLICKED (默认 1.0)
/// 
/// 负向聚类: 使用 status=-1 (点踩)
pub fn update_clusters() -> Result<ClusterResult> {
    // 获取正向反馈数据（点击 + 点赞）
    let pos_data = db::get_vectors_by_statuses(&[
        config::status::CLICKED,
        config::status::LIKED,
    ])?;
    
    // 分离向量和权重
    let mut pos_vectors = Vec::new();
    let mut pos_weights = Vec::new();
    
    for data in &pos_data {
        pos_vectors.push(Array1::from_vec(data.vector.clone()));
        // 根据状态赋予不同权重
        let weight = if data.status == config::status::LIKED {
            config::WEIGHT_LIKED
        } else {
            config::WEIGHT_CLICKED
        };
        pos_weights.push(weight);
    }
    
    // 获取负向反馈数据（点踩）
    let neg_data = db::get_vectors_by_statuses(&[config::status::DISLIKED])?;
    let neg_vectors: Vec<Array1<f32>> = neg_data
        .iter()
        .map(|d| Array1::from_vec(d.vector.clone()))
        .collect();
    
    tracing::info!(
        "🎯 聚类数据: 正向={} (点赞={}, 点击={}), 负向={}",
        pos_vectors.len(),
        pos_data.iter().filter(|d| d.status == config::status::LIKED).count(),
        pos_data.iter().filter(|d| d.status == config::status::CLICKED).count(),
        neg_vectors.len()
    );
    
    let pos_centroids = cluster_vectors_weighted(&pos_vectors, &pos_weights)?;
    let neg_centroids = cluster_vectors(&neg_vectors)?;
    
    // 保存到数据库
    if !pos_centroids.is_empty() {
        db::save_clusters("positive", &pos_centroids)?;
        tracing::info!("✅ 正向聚类生成: {} 个", pos_centroids.len());
    }
    
    if !neg_centroids.is_empty() {
        db::save_clusters("negative", &neg_centroids)?;
        tracing::info!("✅ 负向聚类生成: {} 个", neg_centroids.len());
    }
    
    Ok(ClusterResult {
        pos_centroids,
        neg_centroids,
        pos_count: pos_vectors.len(),
        neg_count: neg_vectors.len(),
    })
}

/// 对向量进行带权重的 K-Means 聚类
fn cluster_vectors_weighted(
    vectors: &[Array1<f32>],
    weights: &[f32],
) -> Result<Vec<Array1<f32>>> {
    // 获取用户设置
    let settings = settings::get_settings().unwrap_or_default();
    
    if vectors.len() < config::CLUSTER_TRIGGER_THRESHOLD {
        return Ok(vec![]);
    }
    
    let n_clusters = settings.pos_clusters.min(vectors.len());
    
    // 转换为 Array2<f32>
    let dim = vectors[0].len();
    let mut data = Array2::<f32>::zeros((vectors.len(), dim));
    for (i, v) in vectors.iter().enumerate() {
        for (j, &val) in v.iter().enumerate() {
            data[[i, j]] = val;
        }
    }
    
    let weight_array = Array1::from_vec(weights.to_vec());
    
    // 创建数据集
    let dataset = DatasetBase::from(data).with_weights(weight_array);
    
    // 运行 K-Means
    let model = KMeans::params(n_clusters).fit(&dataset)?;
    
    // 提取聚类中心
    let centroids: Vec<Array1<f32>> = model
        .centroids()
        .outer_iter()
        .map(|row| row.to_owned())
        .collect();
    
    Ok(centroids)
}

/// 对向量进行无权重的 K-Means 聚类
fn cluster_vectors(vectors: &[Array1<f32>]) -> Result<Vec<Array1<f32>>> {
    // 获取用户设置
    let settings = settings::get_settings().unwrap_or_default();
    
    if vectors.len() < config::CLUSTER_TRIGGER_THRESHOLD {
        return Ok(vec![]);
    }
    
    let n_clusters = settings.neg_clusters.min(vectors.len());
    
    // 转换为 Array2<f32>
    let dim = vectors[0].len();
    let mut data = Array2::<f32>::zeros((vectors.len(), dim));
    for (i, v) in vectors.iter().enumerate() {
        for (j, &val) in v.iter().enumerate() {
            data[[i, j]] = val;
        }
    }
    
    let dataset = DatasetBase::from(data);
    
    // 运行 K-Means
    let model = KMeans::params(n_clusters).fit(&dataset)?;
    
    // 提取聚类中心
    let centroids: Vec<Array1<f32>> = model
        .centroids()
        .outer_iter()
        .map(|row| row.to_owned())
        .collect();
    
    Ok(centroids)
}

/// 重新计算所有未读文章的分数
pub fn recalculate_all_scores() -> Result<usize> {
    // 获取用户设置的 alpha
    let settings = settings::get_settings().unwrap_or_default();
    let alpha = settings.negative_alpha;
    
    let pos_centroids = db::load_clusters("positive")?;
    let neg_centroids = db::load_clusters("negative")?;
    
    tracing::info!(
        "📊 加载聚类: 正向={}, 负向={}, α={}",
        pos_centroids.len(),
        neg_centroids.len(),
        alpha
    );
    
    if pos_centroids.is_empty() {
        tracing::warn!("⚠️ 没有正向聚类，分数无法计算");
        return Ok(0);
    }
    
    // 获取所有未读文章（status = 0）且有向量
    let unread_data = db::get_vectors_by_statuses(&[config::status::UNREAD])?;
    
    let mut scores = Vec::new();
    for data in &unread_data {
        let vector = Array1::from_vec(data.vector.clone());
        let score = super::compute_score_with_alpha(&vector, &pos_centroids, &neg_centroids, alpha);
        scores.push((data.id.clone(), score));
    }
    
    db::update_articles_scores(&scores)?;
    
    tracing::info!("📈 已更新 {} 篇文章的分数", scores.len());
    Ok(scores.len())
}
