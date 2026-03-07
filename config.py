import os

DEBUG = os.environ.get("DEBUG", "false").lower() == "true"
SILICONFLOW_API_KEY = os.environ.get("SILICONFLOW_API_KEY")
SILICONFLOW_API_URL = "https://api.siliconflow.cn/v1/embeddings"
EMBEDDING_MODEL = "BAAI/bge-m3"

DEBUG_MAX_ARTICLES = 30

BAIDU_FANYI_APPID = os.environ.get("BAIDU_FANYI_APPID")
BAIDU_FANYI_APPKEY = os.environ.get("BAIDU_FANYI_APPKEY")

ARXIV_CATEGORIES = ["cs.SD", "cs.AI", "cs.LG", "cs.CV"]

RSS_FEEDS = [f"https://rss.arxiv.org/rss/{cat}" for cat in ARXIV_CATEGORIES]

MAX_CLUSTERS = 10
CLUSTER_TRIGGER_THRESHOLD = 5
DIVERSITY_RATIO = 0.3

# 负向惩罚系数 (α > 1 时对不感兴趣的内容更敏感)
NEGATIVE_PENALTY_ALPHA = 1.5

# 反馈权重
WEIGHT_LIKED = 2.0  # 点赞权重
WEIGHT_CLICKED = 1.0  # 点击权重

DB_PATH = "zenflow.db"
