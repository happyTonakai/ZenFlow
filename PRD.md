# PRD: 极简 AI 论文与新闻个性化推荐 Agent

## 1. 项目愿景

构建一个基于 RSS 订阅的个人信息流 Agent，通过用户简单的“点赞、点开、点踩”行为，利用向量空间聚类算法，实现每日推荐内容的自动进化。

## 2. 核心技术栈

* **语言**: Python 3.10+
* **存储**: SQLite (本地单文件)
* **UI 框架**: Streamlit (快速 Web 交互)
* **向量计算**: NumPy + Scikit-learn (K-Means 聚类)
* **Embedding**: 调用外部 API (<https://docs.siliconflow.cn/cn/api-reference/embeddings/create-embeddings>, modelname = BAAI/bge-m3)
* **抓取**: `feedparser` (处理 RSS/Arxiv)

---

## 3. 数据模型设计 (SQLite)

### Table: `articles`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `id` | TEXT (PK) | 链接的 MD5 或原始 URL |
| `title` | TEXT | 文章标题 |
| `link` | TEXT | 原始链接 |
| `abstract` | TEXT | 摘要内容 |
| `source` | TEXT | 'news' 或 'arxiv' |
| `vector` | BLOB | NumPy 数组序列化后的二进制数据 |
| `status` | INT | 0:未读, 1:已点击, 2:点赞, -1:点踩 |
| `score` | FLOAT | 推荐算法计算出的匹配分 |
| `timestamp` | DATETIME | 抓取时间 (用于滑动窗口清理) |

### Table: `clusters`

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `type` | TEXT | 'positive' 或 'negative' |
| `centroid` | BLOB | 聚类中心的向量数据 |

---

## 4. 核心功能需求

### F1: 数据抓取与向量化 (Ingestion)

1. 从预设的 RSS 列表（新闻 + Arxiv）定时抓取。
2. **去重**: 若 URL 已存在于 `articles` 表，则跳过。
3. **向量化**: 调用 Embedding API 将 `title + abstract` 转化为向量。
4. **存入**: 初始 `status=0`。

### F2: 推荐打分逻辑 (Ranking)

1. 从 `clusters` 表读取 10 个正向中心和 10 个负向中心。
2. 对于新抓取的文章向量 $V_{new}$，计算：

* $S_{pos} = \max(\text{CosineSim}(V_{new}, \text{PositiveCentroids}))$
* $S_{neg} = \max(\text{CosineSim}(V_{new}, \text{NegativeCentroids}))$
* $\text{FinalScore} = S_{pos} - S_{neg}$

3. 更新 `articles` 表中的 `score` 字段。

### F3: 兴趣演化 (Clustering)

1. **触发条件**: 累计新增 10 次交互行为。
2. **聚类算法**:

* 提取所有 `status > 0` 的向量，用 K-Means 聚合成最多 10 个中心。
* 提取所有 `status < 0` 的向量，聚合成最多 10 个中心。

3. **持久化**: 清空并更新 `clusters` 表。

### F4: 滑动窗口管理 (Cleanup)

1. 每日启动时执行：`DELETE FROM articles WHERE timestamp < datetime('now', '-30 days')`。
2. 确保系统不会因为旧数据堆积而变慢。

### F5: 交互界面 (UI)

1. **Tab 分类**: “今日推荐”、“新闻流”、“论文集”。
2. **排序**: 默认按 `score` 降序排列。
3. **操作按钮**:

* 点击标题: 跳转链接，后台更新 `status=1`。
* 👍 按钮: 更新 `status=2`。
* 👎 按钮: 更新 `status=-1`。

---

## 5. 算法伪代码 (供 Coding Agent 参考)

```python
def update_interest_clusters():
    # 正向行为：点赞(2)和点开(1)
    pos_vectors = db.query("SELECT vector FROM articles WHERE status > 0")
    if len(pos_vectors) >= 10:
        centroids = KMeans(n_clusters=10).fit(pos_vectors).cluster_centers_
        db.save_clusters('positive', centroids)

def calculate_article_score(article_vector):
    pos_centroids = db.load_clusters('positive')
    neg_centroids = db.load_clusters('negative')
    
    # 使用 NumPy 计算余弦相似度
    p_sim = max([dot(article_vector, c) for c in pos_centroids]) if pos_centroids else 0
    n_sim = max([dot(article_vector, c) for c in neg_centroids]) if neg_centroids else 0
    
    return p_sim - n_sim

```

---

## 6. 项目文件结构建议

* `main.py`: Streamlit 应用入口。
* `engine.py`: 处理 RSS 抓取、API 调用和数据库交互。
* `algorithm.py`: 聚类与相似度计算逻辑。
* `config.py`: 存放 RSS 地址列表和 API Key。
* `schema.sql`: 数据库初始化脚本。

---
