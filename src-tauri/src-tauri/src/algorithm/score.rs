//! 推荐分数计算

use ndarray::Array1;

use crate::config;

/// 计算文章推荐分数
/// 
/// 公式: FinalScore = MaxSim(正向) - α * MaxSim(负向)
/// 其中 α > 1 表示对不喜欢的内容更敏感
pub fn compute_score(
    article_vector: &Array1<f32>,
    pos_centroids: &[Array1<f32>],
    neg_centroids: &[Array1<f32>],
) -> f32 {
    compute_score_with_alpha(
        article_vector,
        pos_centroids,
        neg_centroids,
        config::NEGATIVE_PENALTY_ALPHA,
    )
}

/// 带自定义 alpha 参数的分数计算
pub fn compute_score_with_alpha(
    article_vector: &Array1<f32>,
    pos_centroids: &[Array1<f32>],
    neg_centroids: &[Array1<f32>],
    alpha: f32,
) -> f32 {
    if pos_centroids.is_empty() {
        return 0.0;
    }

    // 计算与正向聚类中心的最大相似度
    let p_sim = pos_centroids
        .iter()
        .map(|c| article_vector.dot(c))
        .fold(f32::NEG_INFINITY, |a, b| a.max(b));

    // 计算与负向聚类中心的最大相似度
    let n_sim = if neg_centroids.is_empty() {
        0.0
    } else {
        neg_centroids
            .iter()
            .map(|c| article_vector.dot(c))
            .fold(f32::NEG_INFINITY, |a, b| a.max(b))
    };

    p_sim - alpha * n_sim
}

/// 批量计算文章分数
pub fn compute_scores(
    article_vectors: &[(String, Array1<f32>)],
    pos_centroids: &[Array1<f32>],
    neg_centroids: &[Array1<f32>],
) -> Vec<(String, f32)> {
    article_vectors
        .iter()
        .map(|(id, vector)| {
            let score = compute_score(vector, pos_centroids, neg_centroids);
            (id.clone(), score)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::array;

    #[test]
    fn test_compute_score_basic() {
        let article = array![1.0_f32, 0.0, 0.0];
        let pos_centroids = vec![array![1.0_f32, 0.0, 0.0]];
        let neg_centroids = vec![array![0.0_f32, 1.0, 0.0]];
        
        // p_sim = 1.0, n_sim = 0.0, score = 1.0 - 1.5 * 0 = 1.0
        let score = compute_score(&article, &pos_centroids, &neg_centroids);
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_compute_score_with_negative() {
        let article = array![0.5_f32, 0.5, 0.0];
        let pos_centroids = vec![array![1.0_f32, 0.0, 0.0]]; // p_sim = 0.5
        let neg_centroids = vec![array![0.0_f32, 1.0, 0.0]]; // n_sim = 0.5
        
        // score = 0.5 - 1.5 * 0.5 = 0.5 - 0.75 = -0.25
        let score = compute_score(&article, &pos_centroids, &neg_centroids);
        assert!((score - (-0.25)).abs() < 1e-6);
    }

    #[test]
    fn test_empty_pos_centroids() {
        let article = array![1.0_f32, 0.0, 0.0];
        let pos_centroids: Vec<Array1<f32>> = vec![];
        let neg_centroids = vec![array![0.0_f32, 1.0, 0.0]];
        
        let score = compute_score(&article, &pos_centroids, &neg_centroids);
        assert!((score - 0.0).abs() < 1e-6);
    }
}
