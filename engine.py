import hashlib
import logging
import sqlite3
import time
import xml.etree.ElementTree as ET
from typing import Any, Dict, List, Optional

import numpy as np
import requests

import config

logger = logging.getLogger(__name__)

_articles_cache: List[Dict[str, Any]] = []
_translation_cache: Dict[str, tuple] = {}  # {article_id: (translated_text, timestamp)}
TRANSLATION_CACHE_TTL = 3600  # 1 hour


def load_today_articles() -> None:
    global _articles_cache
    conn = get_db()
    try:
        cursor = conn.execute(
            """SELECT id, title, link, abstract, source, status, score, timestamp,
                      translated_abstract, hf_upvotes, ax_upvotes, ax_downvotes
               FROM articles ORDER BY score DESC"""
        )
        columns = [desc[0] for desc in cursor.description]
        _articles_cache = [dict(zip(columns, row)) for row in cursor.fetchall()]
        logger.info(f"📦 已加载 {len(_articles_cache)} 篇文章到内存")
    finally:
        conn.close()


def get_cached_articles() -> List[Dict[str, Any]]:
    return _articles_cache


def init_db() -> sqlite3.Connection:
    conn = sqlite3.connect(config.DB_PATH)
    with open("schema.sql", "r") as f:
        conn.executescript(f.read())
    conn.commit()
    return conn


def get_db() -> sqlite3.Connection:
    return sqlite3.connect(config.DB_PATH)


def _compute_id(url: str) -> str:
    return hashlib.md5(url.encode()).hexdigest()


def fetch_feeds() -> List[Dict[str, Any]]:
    articles_dict: Dict[str, Dict[str, Any]] = {}
    for feed_url in config.RSS_FEEDS:
        try:
            response = requests.get(feed_url, timeout=30)
            response.raise_for_status()
            root = ET.fromstring(response.content)
            channel = root.find("channel")
            if channel is None:
                continue
            ns = {"arxiv": "http://arxiv.org/schemas/atom"}
            for item in channel.findall("item"):
                link_elem = item.find("link")
                title_elem = item.find("title")
                desc_elem = item.find("description")
                announce_elem = item.find("arxiv:announce_type", ns)
                if link_elem is not None and title_elem is not None:
                    announce_type = (
                        announce_elem.text if announce_elem is not None else ""
                    )
                    if announce_type not in ("new", "cross"):
                        continue
                    link_text = link_elem.text or ""
                    title_text = title_elem.text or ""
                    abstract_text = ""
                    if desc_elem is not None and desc_elem.text:
                        abstract_text = desc_elem.text
                        if "Abstract:" in abstract_text:
                            abstract_text = abstract_text.split("Abstract:", 1)[1]
                        abstract_text = abstract_text[:2000]
                    arxiv_id = (
                        link_text.split("/abs/")[-1] if "/abs/" in link_text else ""
                    )
                    article_id = arxiv_id or _compute_id(link_text)
                    if article_id not in articles_dict:
                        articles_dict[article_id] = {
                            "id": article_id,
                            "title": title_text,
                            "link": link_text,
                            "abstract": abstract_text,
                            "source": "arxiv",
                        }
        except Exception as e:
            logger.error(f"Error fetching {feed_url}: {e}")
    return list(articles_dict.values())


def fetch_arxiv_by_ids(arxiv_ids: List[str]) -> List[Dict[str, Any]]:
    articles = []
    if not arxiv_ids:
        return articles
    try:
        id_list = ",".join(arxiv_ids)
        url = f"https://export.arxiv.org/api/query?id_list={id_list}"
        response = requests.get(url, timeout=30)
        response.raise_for_status()
        root = ET.fromstring(response.content)
        ns = {"atom": "http://www.w3.org/2005/Atom"}
        for entry in root.findall("atom:entry", ns):
            title = entry.find("atom:title", ns)
            summary = entry.find("atom:summary", ns)
            # 获取 arXiv ID: 优先从 <id> 元素获取
            id_elem = entry.find("atom:id", ns)
            arxiv_id = ""
            link_text = ""
            if id_elem is not None and id_elem.text:
                link_text = id_elem.text
                # 从 http://arxiv.org/abs/2506.14724v2 提取 ID
                if "/abs/" in link_text:
                    arxiv_id = link_text.split("/abs/")[-1].split("v")[0]
            # 或者从 <link href="..."> 获取
            if not arxiv_id:
                for link in entry.findall("atom:link", ns):
                    href = link.get("href", "")
                    if "/abs/" in href and "pdf" not in href:
                        link_text = href
                        arxiv_id = href.split("/abs/")[-1].split("v")[0]
                        break
            if title is not None and arxiv_id:
                articles.append(
                    {
                        "id": arxiv_id,
                        "title": (title.text or "").replace("\n", " "),
                        "link": link_text or f"https://arxiv.org/abs/{arxiv_id}",
                        "abstract": (summary.text or "")[:2000]
                        if summary is not None
                        else "",
                        "source": "arxiv",
                    }
                )
    except Exception as e:
        logger.error(f"Error fetching arxiv IDs: {e}")
    return articles


def embed_text(text: str) -> Optional[np.ndarray]:
    time.sleep(0.03)
    if not config.SILICONFLOW_API_KEY:
        logger.warning("SILICONFLOW_API_KEY not set")
        return None
    headers = {
        "Authorization": f"Bearer {config.SILICONFLOW_API_KEY}",
        "Content-Type": "application/json",
    }
    payload = {
        "model": config.EMBEDDING_MODEL,
        "input": text[:8000],
    }
    try:
        response = requests.post(
            config.SILICONFLOW_API_URL,
            headers=headers,
            json=payload,
            timeout=30,
        )
        response.raise_for_status()
        data = response.json()
        embedding = data["data"][0]["embedding"]
        return np.array(embedding, dtype=np.float32)
    except Exception as e:
        logger.error(f"Embedding error: {e}")
        return None


def translate(text: str) -> Optional[str]:
    if not config.BAIDU_FANYI_APPID or not config.BAIDU_FANYI_APPKEY:
        logger.warning("BAIDU_FANYI_APPID or BAIDU_FANYI_APPKEY not set")
        return None
    for attempt in range(3):
        try:
            salt = str(int(time.time()))
            sign_str = f"{config.BAIDU_FANYI_APPID}{text[:6000]}{salt}{config.BAIDU_FANYI_APPKEY}"
            sign = hashlib.md5(sign_str.encode("utf-8")).hexdigest()
            params = {
                "q": text[:6000],
                "from": "en",
                "to": "zh",
                "appid": config.BAIDU_FANYI_APPID,
                "salt": salt,
                "sign": sign,
            }
            response = requests.get(
                "https://fanyi-api.baidu.com/api/trans/vip/translate",
                params=params,
                timeout=10,
            )
            data = response.json()
            if "trans_result" in data:
                return "".join(t["dst"] for t in data["trans_result"])
            return None
        except Exception as e:
            logger.error(f"Translation error (attempt {attempt + 1}): {e}")
            if attempt < 2:
                time.sleep(1 * (attempt + 1))
    return None


def save_article(
    article: Dict[str, Any],
    vector: Optional[np.ndarray],
    translated_abstract: Optional[str] = None,
) -> None:
    conn = get_db()
    try:
        conn.execute(
            """INSERT OR IGNORE INTO articles 
               (id, title, link, abstract, source, vector, status, score, translated_abstract)
               VALUES (?, ?, ?, ?, ?, ?, 0, 0.0, ?)""",
            (
                article["id"],
                article["title"],
                article["link"],
                article["abstract"],
                article["source"],
                vector.tobytes() if vector is not None else None,
                translated_abstract,
            ),
        )
        conn.commit()
    finally:
        conn.close()


def update_article_status(article_id: str, status: int) -> None:
    conn = get_db()
    try:
        conn.execute(
            "UPDATE articles SET status = ? WHERE id = ?",
            (status, article_id),
        )
        conn.commit()
        for art in _articles_cache:
            if art["id"] == article_id:
                art["status"] = status
                break
    finally:
        conn.close()


def get_articles(
    status: Optional[int] = None,
    limit: int = 50,
    order_by: str = "score DESC",
    include_vector: bool = False,
) -> List[Dict[str, Any]]:
    conn = get_db()
    try:
        columns = "id, title, link, abstract, source, status, score, timestamp"
        if include_vector:
            columns += ", vector"
        query = f"SELECT {columns} FROM articles"
        params = []
        if status is not None:
            query += " WHERE status = ?"
            params.append(status)
        query += f" ORDER BY {order_by} LIMIT ?"
        params.append(limit)
        cursor = conn.execute(query, params)
        rows = cursor.fetchall()
        columns = [desc[0] for desc in cursor.description]
        return [dict(zip(columns, row)) for row in rows]
    finally:
        conn.close()


def get_article(article_id: str) -> Optional[Dict[str, Any]]:
    conn = get_db()
    try:
        cursor = conn.execute(
            "SELECT * FROM articles WHERE id = ?",
            (article_id,),
        )
        row = cursor.fetchone()
        if row is None:
            return None
        columns = [desc[0] for desc in cursor.description]
        return dict(zip(columns, row))
    finally:
        conn.close()


def ensure_translated(article: Dict[str, Any]) -> Dict[str, Any]:
    if article.get("translated_abstract"):
        return article

    article_id = article.get("id")
    if article_id and article_id in _translation_cache:
        cached_text, timestamp = _translation_cache[article_id]
        if time.time() - timestamp < TRANSLATION_CACHE_TTL:
            article["translated_abstract"] = cached_text
            return article

    if not article.get("abstract"):
        return article
    translated = translate(article["abstract"])
    if translated:
        conn = get_db()
        try:
            conn.execute(
                "UPDATE articles SET translated_abstract = ? WHERE id = ?",
                (translated, article["id"]),
            )
            conn.commit()
            article["translated_abstract"] = translated
            if article_id:
                _translation_cache[article_id] = (translated, time.time())
        finally:
            conn.close()
    return article


def get_article_vectors(status: int) -> List[tuple]:
    conn = get_db()
    try:
        cursor = conn.execute(
            "SELECT id, vector, status FROM articles WHERE status = ? AND vector IS NOT NULL",
            (status,),
        )
        return cursor.fetchall()
    finally:
        conn.close()


def get_article_vectors_by_statuses(statuses: List[int]) -> List[tuple]:
    """获取多个状态的文章向量，返回 (id, vector, status)"""
    conn = get_db()
    try:
        placeholders = ",".join("?" * len(statuses))
        cursor = conn.execute(
            f"SELECT id, vector, status FROM articles WHERE status IN ({placeholders}) AND vector IS NOT NULL",
            statuses,
        )
        return cursor.fetchall()
    finally:
        conn.close()


def get_existing_article_ids(article_ids: List[str]) -> set:
    """批量检查文章ID是否已存在数据库中"""
    if not article_ids:
        return set()
    conn = get_db()
    try:
        placeholders = ",".join("?" * len(article_ids))
        cursor = conn.execute(
            f"SELECT id FROM articles WHERE id IN ({placeholders})",
            article_ids,
        )
        return {row[0] for row in cursor.fetchall()}
    finally:
        conn.close()


def save_clusters(cluster_type: str, centroids: np.ndarray) -> None:
    conn = get_db()
    try:
        conn.execute("DELETE FROM clusters WHERE type = ?", (cluster_type,))
        for centroid in centroids:
            conn.execute(
                "INSERT INTO clusters (type, centroid) VALUES (?, ?)",
                (cluster_type, centroid.tobytes()),
            )
        conn.commit()
    finally:
        conn.close()


def load_clusters(cluster_type: str) -> List[np.ndarray]:
    conn = get_db()
    try:
        cursor = conn.execute(
            "SELECT centroid FROM clusters WHERE type = ?",
            (cluster_type,),
        )
        return [np.frombuffer(row[0], dtype=np.float32) for row in cursor.fetchall()]
    finally:
        conn.close()


def clean_old_articles(days: int = 30) -> None:
    conn = get_db()
    try:
        conn.execute(
            "DELETE FROM articles WHERE timestamp < datetime('now', ?)",
            (f"-{days} days",),
        )
        conn.commit()
    finally:
        conn.close()


def get_article_count_by_status() -> Dict[int, int]:
    conn = get_db()
    try:
        cursor = conn.execute("SELECT status, COUNT(*) FROM articles GROUP BY status")
        return dict(cursor.fetchall())
    finally:
        conn.close()


def get_alphaxiv_votes(arxiv_id: str) -> Optional[Dict[str, int]]:
    """
    获取 AlphaXiv 上的论文投票数据

    Returns:
        {"upvotes": int, "downvotes": int, "questions": int} or None
    """
    import re

    try:
        # 移除版本后缀
        clean_id = re.sub(r"v\d+$", "", arxiv_id)
        url = f"https://www.alphaxiv.org/abs/{clean_id}"
        response = requests.get(url, timeout=10)
        response.raise_for_status()

        # 提取 metrics 数据
        match = re.search(
            r"questions_count:(\d+),upvotes_count:(\d+),downvotes_count:(\d+)",
            response.text,
        )
        if match:
            return {
                "questions": int(match.group(1)),
                "upvotes": int(match.group(2)),
                "downvotes": int(match.group(3)),
            }
        return None
    except Exception as e:
        logger.debug(f"AlphaXiv fetch failed for {arxiv_id}: {e}")
        return None


def get_huggingface_votes(arxiv_id: str) -> Optional[int]:
    """
    获取 HuggingFace Papers 上的点赞数

    Returns:
        upvotes count or None
    """
    import re

    try:
        # 移除版本后缀
        clean_id = re.sub(r"v\d+$", "", arxiv_id)
        url = f"https://huggingface.co/papers/{clean_id}"
        response = requests.get(url, timeout=10)
        response.raise_for_status()

        # 提取 upvotes 数据
        match = re.search(r"&quot;upvotes&quot;:(\d+)", response.text)
        if match:
            return int(match.group(1))
        return None
    except Exception as e:
        logger.debug(f"HuggingFace fetch failed for {arxiv_id}: {e}")
        return None


def get_community_votes(arxiv_id: str) -> Dict[str, Any]:
    """
    获取社区投票数据（AlphaXiv + HuggingFace）

    Returns:
        {
            "alphaxiv": {"upvotes": x, "downvotes": y} or None,
            "huggingface": z or None
        }
    """
    result = {"alphaxiv": None, "huggingface": None}

    # 并行获取两个平台的数据
    try:
        result["alphaxiv"] = get_alphaxiv_votes(arxiv_id)
    except Exception:
        pass

    try:
        result["huggingface"] = get_huggingface_votes(arxiv_id)
    except Exception:
        pass

    return result


def update_article_votes(article_id: str) -> Optional[Dict[str, Any]]:
    """
    更新文章的社区投票数据并保存到数据库

    Returns:
        更新后的投票数据 or None
    """
    votes = get_community_votes(article_id)
    if not votes:
        return None

    hf_upvotes = votes.get("huggingface")
    ax = votes.get("alphaxiv")
    ax_upvotes = ax.get("upvotes") if ax else None
    ax_downvotes = ax.get("downvotes") if ax else None

    conn = get_db()
    try:
        conn.execute(
            """UPDATE articles
               SET hf_upvotes = ?, ax_upvotes = ?, ax_downvotes = ?,
                   votes_updated_at = CURRENT_TIMESTAMP
               WHERE id = ?""",
            (hf_upvotes, ax_upvotes, ax_downvotes, article_id),
        )
        conn.commit()
    finally:
        conn.close()

    # 更新缓存
    for art in _articles_cache:
        if art["id"] == article_id:
            art["hf_upvotes"] = hf_upvotes
            art["ax_upvotes"] = ax_upvotes
            art["ax_downvotes"] = ax_downvotes
            break

    return votes


def update_votes_for_articles(article_ids: List[str]) -> Dict[str, Any]:
    """
    批量更新多篇文章的投票数据

    Returns:
        {"updated": int, "failed": int}
    """
    updated = 0
    failed = 0

    for aid in article_ids:
        try:
            result = update_article_votes(aid)
            if result:
                updated += 1
            else:
                failed += 1
        except Exception as e:
            logger.debug(f"Failed to update votes for {aid}: {e}")
            failed += 1

    return {"updated": updated, "failed": failed}


def get_liked_count() -> int:
    """获取已标记为喜欢(status=2)的文章数量"""
    conn = get_db()
    try:
        cursor = conn.execute(
            "SELECT COUNT(*) FROM articles WHERE status = 2",
        )
        return cursor.fetchone()[0]
    finally:
        conn.close()


def is_initialized() -> bool:
    """检查是否已完成初始化(至少有5篇偏好文章)"""
    return get_liked_count() >= config.CLUSTER_TRIGGER_THRESHOLD
