import os

SILICONFLOW_API_KEY = os.environ.get("SILICONFLOW_API_KEY")
SILICONFLOW_API_URL = "https://api.siliconflow.cn/v1/embeddings"
EMBEDDING_MODEL = "BAAI/bge-m3"

BAIDU_FANYI_APPID = os.environ.get("BAIDU_FANYI_APPID")
BAIDU_FANYI_APPKEY = os.environ.get("BAIDU_FANYI_APPKEY")

ARXIV_CATEGORIES = ["cs.SD", "cs.AI", "cs.LG", "cs.CV"]

RSS_FEEDS = [f"https://rss.arxiv.org/rss/{cat}" for cat in ARXIV_CATEGORIES]

MAX_CLUSTERS = 10
CLUSTER_TRIGGER_THRESHOLD = 10
DIVERSITY_RATIO = 0.3

DB_PATH = "zenflow.db"
