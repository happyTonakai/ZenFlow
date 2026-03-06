import numpy as np
from numpy.typing import NDArray
from sklearn.cluster import KMeans
from typing import Dict, List

import config
import engine


def compute_score(
    article_vector: NDArray[np.float32],
    pos_centroids: List[NDArray[np.float32]],
    neg_centroids: List[NDArray[np.float32]],
) -> float:
    if len(pos_centroids) == 0:
        return 0.0

    p_sim = max(np.dot(article_vector, c) for c in pos_centroids)
    n_sim = (
        max(np.dot(article_vector, c) for c in neg_centroids) if neg_centroids else 0.0
    )
    return float(p_sim - n_sim)


def update_clusters() -> None:
    pos_vectors = []
    neg_vectors = []

    for row in engine.get_article_vectors(status=2):
        vec = np.frombuffer(row[1], dtype=np.float32)
        pos_vectors.append(vec)

    for row in engine.get_article_vectors(status=-1):
        vec = np.frombuffer(row[1], dtype=np.float32)
        neg_vectors.append(vec)

    pos_centroids = []
    neg_centroids = []

    if len(pos_vectors) >= config.CLUSTER_TRIGGER_THRESHOLD:
        n_clusters = min(config.MAX_CLUSTERS, len(pos_vectors))
        kmeans = KMeans(n_clusters=n_clusters, random_state=42, n_init=10)
        kmeans.fit(pos_vectors)
        pos_centroids = kmeans.cluster_centers_
        engine.save_clusters("positive", pos_centroids)

    if len(neg_vectors) >= config.CLUSTER_TRIGGER_THRESHOLD:
        n_clusters = min(config.MAX_CLUSTERS, len(neg_vectors))
        kmeans = KMeans(n_clusters=n_clusters, random_state=42, n_init=10)
        kmeans.fit(neg_vectors)
        neg_centroids = kmeans.cluster_centers_
        engine.save_clusters("negative", neg_centroids)


def calculate_all_scores() -> None:
    pos_centroids = engine.load_clusters("positive")
    neg_centroids = engine.load_clusters("negative")

    if not pos_centroids:
        return

    articles = engine.get_articles(status=0, limit=100)
    for article in articles:
        if article["vector"] is None:
            continue
        vec = np.frombuffer(article["vector"], dtype=np.float32)
        score = compute_score(vec, pos_centroids, neg_centroids)
        conn = engine.get_db()
        try:
            conn.execute(
                "UPDATE articles SET score = ? WHERE id = ?",
                (score, article["id"]),
            )
            conn.commit()
        finally:
            conn.close()


def get_diverse_recommendations(limit: int = 20) -> List[Dict]:
    diversity_count = int(limit * config.DIVERSITY_RATIO)
    score_count = limit - diversity_count

    scored = engine.get_articles(limit=100, order_by="score DESC")
    scored = [a for a in scored if a["status"] == 0]

    if len(scored) <= limit:
        return scored

    score_based = scored[:score_count]

    ids_in_score = {a["id"] for a in score_based}
    remaining = [a for a in scored if a["id"] not in ids_in_score]
    import random

    random.shuffle(remaining)
    diverse = remaining[:diversity_count]

    result = score_based + diverse
    random.shuffle(result)
    return result
