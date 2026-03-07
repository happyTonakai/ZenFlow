import logging
import numpy as np
from numpy.typing import NDArray
from sklearn.cluster import KMeans
from typing import Dict, List

import config
import engine

logger = logging.getLogger(__name__)


def compute_score(
    article_vector: NDArray[np.float32],
    pos_centroids: List[NDArray[np.float32]],
    neg_centroids: List[NDArray[np.float32]],
    alpha: float = 1.5,
) -> float:
    """
    计算文章推荐分数

    公式: FinalScore = MaxSim(正向) - α * MaxSim(负向)
    其中 α > 1 表示对不喜欢的内容更敏感
    """
    if len(pos_centroids) == 0:
        return 0.0

    p_sim = max(np.dot(article_vector, c) for c in pos_centroids)
    n_sim = (
        max(np.dot(article_vector, c) for c in neg_centroids) if neg_centroids else 0.0
    )
    return float(p_sim - alpha * n_sim)


def update_clusters() -> None:
    """
    更新聚类中心

    正向聚类: 使用 status=1 (点击) 和 status=2 (点赞)
    - 点赞权重 = WEIGHT_LIKED (默认 2.0)
    - 点击权重 = WEIGHT_CLICKED (默认 1.0)

    负向聚类: 使用 status=-1 (点踩)
    """
    # 获取正向反馈数据（点击 + 点赞）
    pos_data = engine.get_article_vectors_by_statuses([1, 2])
    pos_vectors = []
    pos_weights = []

    for row in pos_data:
        vec = np.frombuffer(row[1], dtype=np.float32)
        status = row[2]
        pos_vectors.append(vec)
        # 根据状态赋予不同权重
        if status == 2:  # 点赞
            pos_weights.append(config.WEIGHT_LIKED)
        else:  # 点击
            pos_weights.append(config.WEIGHT_CLICKED)

    # 获取负向反馈数据（点踩）
    neg_vectors = []
    for row in engine.get_article_vectors(status=-1):
        vec = np.frombuffer(row[1], dtype=np.float32)
        neg_vectors.append(vec)

    logger.info(
        f"🎯 聚类数据: 正向={len(pos_vectors)} (点赞={sum(1 for r in pos_data if r[2] == 2)}, 点击={sum(1 for r in pos_data if r[2] == 1)}), 负向={len(neg_vectors)}"
    )

    pos_centroids = []
    neg_centroids = []

    # 正向聚类（带权重）
    if len(pos_vectors) >= config.CLUSTER_TRIGGER_THRESHOLD:
        n_clusters = min(config.MAX_CLUSTERS, len(pos_vectors))
        kmeans = KMeans(n_clusters=n_clusters, random_state=42, n_init=10)
        kmeans.fit(pos_vectors, sample_weight=pos_weights)
        # KMeans 返回 float64，需要转换为 float32
        pos_centroids = kmeans.cluster_centers_.astype(np.float32)
        engine.save_clusters("positive", pos_centroids)
        logger.info(f"✅ 正向聚类生成: {n_clusters} 个")

    # 负向聚类
    if len(neg_vectors) >= config.CLUSTER_TRIGGER_THRESHOLD:
        n_clusters = min(config.MAX_CLUSTERS, len(neg_vectors))
        kmeans = KMeans(n_clusters=n_clusters, random_state=42, n_init=10)
        kmeans.fit(neg_vectors)
        # KMeans 返回 float64，需要转换为 float32
        neg_centroids = kmeans.cluster_centers_.astype(np.float32)
        engine.save_clusters("negative", neg_centroids)
        logger.info(f"✅ 负向聚类生成: {n_clusters} 个")


def calculate_all_scores() -> None:
    pos_centroids = engine.load_clusters("positive")
    neg_centroids = engine.load_clusters("negative")

    logger.info(
        f"📊 加载聚类: 正向={len(pos_centroids) if pos_centroids else 0}, 负向={len(neg_centroids) if neg_centroids else 0}"
    )

    if not pos_centroids:
        logger.warning("⚠️ 没有正向聚类，分数无法计算")
        return

    articles = engine.get_articles(status=0, limit=1000, include_vector=True)
    for article in articles:
        if article["vector"] is None:
            continue
        vec = np.frombuffer(article["vector"], dtype=np.float32)
        score = compute_score(
            vec, pos_centroids, neg_centroids, config.NEGATIVE_PENALTY_ALPHA
        )
        article["score"] = score
        conn = engine.get_db()
        try:
            conn.execute(
                "UPDATE articles SET score = ? WHERE id = ?",
                (score, article["id"]),
            )
            conn.commit()
        finally:
            conn.close()

    for art in engine._articles_cache:
        db_art = next((a for a in articles if a["id"] == art["id"]), None)
        if db_art:
            art["score"] = db_art["score"]


def get_diverse_recommendations(limit: int = 20, offset: int = 0) -> Dict:
    import engine

    articles = engine.get_cached_articles()
    unread = [a for a in articles if a["status"] == 0]
    return get_diverse_recommendations_from_list(unread, limit=limit, offset=offset)


def get_diverse_recommendations_from_list(
    articles: List[Dict], limit: int = 20, offset: int = 0
) -> Dict:
    """
    返回分组推荐结果

    Returns:
        {
            "score_based": [...],  # 70% 按分数排序的推荐
            "diverse": [...]       # 30% 多样性推荐（也按分数排序）
        }
    """
    if len(articles) <= offset:
        return {"score_based": [], "diverse": []}

    articles = articles[offset:]

    # 确保文章按分数降序排序
    articles = sorted(articles, key=lambda x: x.get("score", 0), reverse=True)

    diversity_count = int(limit * config.DIVERSITY_RATIO)
    score_count = limit - diversity_count

    if len(articles) <= limit:
        return {"score_based": articles, "diverse": []}

    # 前 70% 按分数排序
    score_based = articles[:score_count]

    # 后 30% 从剩余文章中随机选，但仍按分数排序
    ids_in_score = {a["id"] for a in score_based}
    remaining = [a for a in articles if a["id"] not in ids_in_score]
    import random

    random.shuffle(remaining)
    diverse = remaining[:diversity_count]
    # 多样性部分也按分数排序
    diverse = sorted(diverse, key=lambda x: x.get("score", 0), reverse=True)

    return {"score_based": score_based, "diverse": diverse}
