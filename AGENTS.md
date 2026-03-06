# ZenFlow - AI Paper & News Recommendation Agent

## Project Overview

A personal RSS-based recommendation agent that learns user preferences through feedback (like, click, dislike) and uses vector space clustering to recommend daily content.

**Tech Stack:**
- Python 3.10+
- SQLite (local single file)
- Streamlit (Web UI)
- NumPy + Scikit-learn (K-Means clustering)
- SiliconFlow Embedding API (BAAI/bge-m3)
- feedparser (RSS/Arxiv fetching)

## Project Structure

```
ZenFlow/
├── main.py           # Streamlit app entry point
├── engine.py         # RSS fetching, API calls, DB interactions
├── algorithm.py      # Clustering & similarity calculations
├── config.py         # RSS feeds list, API keys
├── schema.sql        # Database initialization
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
uv add streamlit feedparser numpy scikit-learn

# Add dev dependencies
uv add --dev pytest ruff
```

### Run Application
```bash
streamlit run main.py
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

### Streamlit UI
- Keep UI code minimal in `main.py`
- Extract logic to separate modules
- Use `st.cache_data` for expensive operations
- Handle exceptions gracefully with `st.error()`

---

## Core Modules

### config.py
Store API keys in environment variables (never commit):
```python
import os
API_KEY = os.environ.get("SILICONFLOW_API_KEY")
RSS_FEEDS = [...]  # List of RSS URLs
```

### engine.py
- `fetch_feeds()`: Fetch from RSS/Arxiv
- `embed_text(text)`: Call embedding API
- `save_article(article)`: Insert/update article in DB

### algorithm.py
- `compute_score(article_vector, pos_centroids, neg_centroids)`: Cosine similarity
- `update_clusters(vectors, n_clusters)`: K-Means clustering
- `get_recommended_articles(limit=20)`: Fetch top-scored articles

---

## Database Schema

### articles
| Field | Type | Description |
|-------|------|-------------|
| id | TEXT PK | MD5 of URL or original URL |
| title | TEXT | Article title |
| link | TEXT | Original link |
| abstract | TEXT | Summary content |
| source | TEXT | 'news' or 'arxiv' |
| vector | BLOB | NumPy array (serialized) |
| status | INT | 0:unread, 1:clicked, 2:liked, -1:disliked |
| score | FLOAT | Recommendation score |
| timestamp | DATETIME | Fetch time |

### clusters
| Field | Type | Description |
|-------|------|-------------|
| type | TEXT PK | 'positive' or 'negative' |
| centroid | BLOB | Cluster center vector |

---

## Development Workflow

1. **Make changes** in feature branches
2. **Run tests**: `pytest -v`
3. **Format code**: `ruff format .`
4. **Check lint**: `ruff check .`
5. **Test manually**: `streamlit run main.py`

---

## Key Algorithms

### Scoring
```python
def compute_score(article_vector, pos_centroids, neg_centroids):
    p_sim = max(dot(article_vector, c) for c in pos_centroids) if pos_centroids else 0
    n_sim = max(dot(article_vector, c) for c in neg_centroids) if neg_centroids else 0
    return p_sim - n_sim
```

### Clustering Trigger
- Re-cluster when累计 10+ new feedback interactions
- K-Means with up to 10 clusters for positive, 10 for negative

---

## Notes

- All API keys must be set via environment variables
- Database is auto-created on first run
- Sliding window: auto-delete articles older than 30 days
