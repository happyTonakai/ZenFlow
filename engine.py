import hashlib
import sqlite3
import time
import xml.etree.ElementTree as ET
from typing import Any, Dict, List, Optional

import numpy as np
import requests

import config


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
            print(f"Error fetching {feed_url}: {e}")
    return list(articles_dict.values())


def fetch_arxiv_by_ids(arxiv_ids: List[str]) -> List[Dict[str, Any]]:
    articles = []
    if not arxiv_ids:
        return articles
    try:
        id_list = ",".join(arxiv_ids)
        url = f"http://export.arxiv.org/api/query?id_list={id_list}"
        response = requests.get(url, timeout=30)
        response.raise_for_status()
        root = ET.fromstring(response.content)
        for entry in root.findall("entry"):
            title = entry.find("title")
            link = entry.find("link")
            summary = entry.find("summary")
            if title is not None and link is not None:
                link_text = link.text or ""
                articles.append(
                    {
                        "id": link_text.split("/abs/")[-1]
                        if "/abs/" in link_text
                        else "",
                        "title": (title.text or "").replace("\n", " "),
                        "link": link_text,
                        "abstract": (summary.text or "")[:2000]
                        if summary is not None
                        else "",
                        "source": "arxiv",
                    }
                )
    except Exception as e:
        print(f"Error fetching arxiv IDs: {e}")
    return articles


def embed_text(text: str) -> Optional[np.ndarray]:
    time.sleep(0.03)
    if not config.SILICONFLOW_API_KEY:
        print("SILICONFLOW_API_KEY not set")
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
        print(f"Embedding error: {e}")
        return None


def translate(text: str) -> Optional[str]:
    if not config.BAIDU_FANYI_APPID or not config.BAIDU_FANYI_APPKEY:
        print("BAIDU_FANYI_APPID or BAIDU_FANYI_APPKEY not set")
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
            print(f"Translation error (attempt {attempt + 1}): {e}")
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
    finally:
        conn.close()


def get_articles(
    status: Optional[int] = None,
    limit: int = 50,
    order_by: str = "score DESC",
) -> List[Dict[str, Any]]:
    conn = get_db()
    try:
        query = "SELECT * FROM articles"
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
        finally:
            conn.close()
    return article


def get_article_vectors(status: int) -> List[tuple]:
    conn = get_db()
    try:
        cursor = conn.execute(
            "SELECT id, vector FROM articles WHERE status = ?",
            (status,),
        )
        return cursor.fetchall()
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
