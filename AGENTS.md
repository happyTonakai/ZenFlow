# ZenFlow - AI Paper & News Recommendation Agent

**IMPORTANT: Always use `uv run` instead of `python` to run Python commands.**

## Project Overview

A personal RSS-based recommendation agent that learns user preferences through feedback (like, click, dislike) and uses vector space clustering to recommend daily content. Features community voting integration and intelligent scoring with weighted feedback.

**Tech Stack:**
- Python 3.10+
- SQLite (local single file)
- FastAPI + Uvicorn (Web server)
- NumPy + Scikit-learn (K-Means clustering)
- SiliconFlow Embedding API (BAAI/bge-m3)
- feedparser (RSS/Arxiv fetching)

## Project Structure

```
ZenFlow/
├── main.py           # FastAPI app entry point
├── engine.py         # RSS fetching, API calls, DB interactions
├── algorithm.py      # Clustering & similarity calculations
├── config.py         # RSS feeds list, API keys, algorithm params
├── schema.sql        # Database initialization
├── preferences.txt   # Initial preference articles (arXiv URLs)
├── pyproject.toml    # uv project config (auto-generated)
└── zenflow.db        # SQLite database (auto-generated)
```

---

## Build & Test Commands

### Dependencies
```bash
# Initialize project with uv
uv init

# Add dependencies
uv add fastapi uvicorn feedparser numpy scikit-learn requests

# Add dev dependencies
uv add --dev pytest ruff
```

### Run Application
```bash
uv run uvicorn main:app --reload
```

### Lint & Format
```bash
# Run ruff (lint + format)
ruff check .
ruff format .
```

### Run Single Test
```bash
# Using pytest (if tests exist)
pytest tests/test_algorithm.py::TestClassName::test_method_name -v

# Or run a specific test file
pytest tests/test_engine.py -v

# Run with coverage
pytest --cov=src --cov-report=html
```

### Database
```bash
# Initialize database
sqlite3 zenflow.db < schema.sql

# View data
sqlite3 zenflow.db "SELECT * FROM articles LIMIT 10;"
```

---

## Code Style Guidelines

### Imports
- Standard library first, then third-party, then local
- Use absolute imports: `from engine import fetch_feeds`
- Group: `import` statements, then `from` statements
- Sort alphabetically within groups

### Formatting
- **Line length**: Max 100 characters
- **Indentation**: 4 spaces (no tabs)
- **Blank lines**: 2 between top-level definitions, 1 between methods
- Use Ruff for formatting: `ruff format .`

### Types
- Use type hints for all function signatures
- Prefer `Optional[X]` over `X | None`
- Use `List`, `Dict` from `typing` (or Python 3.9+ built-ins)
- Example:
  ```python
  def calculate_score(article_vector: np.ndarray, clusters: List[np.ndarray]) -> float:
      """Calculate similarity score between article and clusters."""
      ...
  ```

### Naming Conventions
- **Functions/variables**: `snake_case` (e.g., `fetch_feeds`, `article_vector`)
- **Classes**: `PascalCase` (e.g., `ArticleFetcher`, `ClusterEngine`)
- **Constants**: `UPPER_SNAKE_CASE` (e.g., `MAX_CLUSTERS = 10`)
- **Private methods**: prefix with `_` (e.g., `_clean_old_articles`)

### Error Handling
- Use specific exceptions, not bare `except`
- Always log errors before raising
- Wrap external API calls with try/except:
  ```python
  try:
      response = call_embedding_api(text)
  except TimeoutError as e:
      logger.error(f"API timeout: {e}")
      raise
  ```
- Use custom exceptions for domain errors:
  ```python
  class ClusterError(Exception):
      """Raised when clustering fails."""
      pass
  ```

### Database Operations
- Use parameterized queries to prevent SQL injection
- Always close connections (use context managers)
- Log slow queries (>1 second)

### Testing
- Test files: `tests/test_*.py`
- Test classes: `TestClassName`
- Test functions: `test_method_name_scenario`
- Use pytest fixtures for common setup
- Aim for 80%+ coverage on core algorithms

### FastAPI Server
- Keep routes minimal in `main.py`
- Extract logic to separate modules
- Use global cache for article data
- Handle exceptions gracefully with HTTP status codes

---

## Core Modules

### config.py
Store API keys in environment variables (never commit):
```python
import os
SILICONFLOW_API_KEY = os.environ.get("SILICONFLOW_API_KEY")
BAIDU_FANYI_APPID = os.environ.get("BAIDU_FANYI_APPID")
BAIDU_FANYI_APPKEY = os.environ.get("BAIDU_FANYI_APPKEY")

# Algorithm parameters
NEGATIVE_PENALTY_ALPHA = 1.5  # 负向惩罚系数
WEIGHT_LIKED = 2.0            # 点赞权重
WEIGHT_CLICKED = 1.0          # 点击权重
DIVERSITY_RATIO = 0.3         # 多样性推荐比例
CLUSTER_TRIGGER_THRESHOLD = 5 # 聚类触发阈值
```

### engine.py
- `fetch_feeds()`: Fetch from RSS/Arxiv
- `fetch_arxiv_by_ids(arxiv_ids)`: Fetch specific arXiv papers by ID
- `embed_text(text)`: Call embedding API
- `save_article(article)`: Insert/update article in DB
- `get_community_votes(arxiv_id)`: Fetch votes from AlphaXiv + HuggingFace
- `is_initialized()`: Check if system has enough preference data (>=5 liked)
- `load_today_articles()`: Load articles into memory cache
- `get_cached_articles()`: Get cached articles from memory

### algorithm.py
- `compute_score(article_vector, pos_centroids, neg_centroids, alpha=1.5)`: Weighted similarity scoring
- `update_clusters()`: K-Means clustering with weighted feedback
- `calculate_all_scores()`: Recalculate all article scores
- `get_diverse_recommendations(limit, offset)`: Get grouped recommendations (70% score + 30% diverse)

---

## Database Schema

### articles
| Field | Type | Description |
|-------|------|-------------|
| id | TEXT PK | arXiv ID (e.g., "2506.14724") |
| title | TEXT | Article title |
| link | TEXT | Original link |
| abstract | TEXT | Summary content |
| source | TEXT | 'news' or 'arxiv' |
| vector | BLOB | NumPy array (serialized, float32) |
| status | INT | 0:unread, 1:clicked, 2:liked, -1:disliked |
| score | FLOAT | Recommendation score |
| translated_abstract | TEXT | Baidu translated abstract |
| hf_upvotes | INT | HuggingFace Papers upvotes |
| ax_upvotes | INT | AlphaXiv upvotes |
| ax_downvotes | INT | AlphaXiv downvotes |
| votes_updated_at | DATETIME | Last vote update time |
| timestamp | DATETIME | Fetch time |

### clusters
| Field | Type | Description |
|-------|------|-------------|
| id | INT PK | Auto-increment ID |
| type | TEXT | 'positive' or 'negative' |
| centroid | BLOB | Cluster center vector (float32) |

---

## Development Workflow

1. **Make changes** in feature branches
2. **Run tests**: `pytest -v`
3. **Format code**: `ruff format .`
4. **Check lint**: `ruff check .`
5. **Test manually**: `uv run uvicorn main:app --reload`

---

## Key Algorithms

### Scoring Formula
```python
def compute_score(article_vector, pos_centroids, neg_centroids, alpha=1.5):
    """
    FinalScore = MaxSim(正向) - α * MaxSim(负向)
    α > 1 表示对不喜欢的内容更敏感
    """
    p_sim = max(dot(article_vector, c) for c in pos_centroids) if pos_centroids else 0
    n_sim = max(dot(article_vector, c) for c in neg_centroids) if neg_centroids else 0
    return p_sim - alpha * n_sim
```

### Weighted Clustering
- **Positive clusters**: Use clicked (status=1, weight=1.0) + liked (status=2, weight=2.0)
- **Negative clusters**: Use disliked (status=-1)
- Trigger when >= 5 feedback articles available
- K-Means with up to 10 clusters per type

### Recommendation Groups
- **70% Score-based**: Top articles by recommendation score
- **30% Diversity**: Random selection from remaining, still sorted by score
- Supports pagination with offset

### Community Voting
- Fetches votes from HuggingFace Papers and AlphaXiv
- Cached in database, can be refreshed on demand
- Display: 🤗 HuggingFace upvotes, 🔬 AlphaXiv net votes

---

## Notes

- All API keys must be set via environment variables
- Database is auto-created on first run
- **Initialization required**: System needs >= 5 liked articles before recommendations work
- Use `preferences.txt` to import initial preference articles (arXiv URLs)
- DEBUG mode: Set `DEBUG=true` environment variable for verbose logging
- Articles are cached in memory for performance
- Translation cache TTL: 1 hour
