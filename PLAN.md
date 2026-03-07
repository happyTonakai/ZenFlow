# ZenFlow Tauri v2 迁移计划

## 项目概述

将 Python 原型 (`python_prototype/`) 迁移至 Tauri v2 + Rust 本地原生应用。

---

## 状态机制设计

| status | 含义 | 聚类参与 |
|--------|------|----------|
| `0` | 未读 | ❌ |
| `1` | 已点击（真正阅读） | ✅ 正向聚类，权重 1.0 |
| `2` | 已点赞 | ✅ 正向聚类，权重 2.0 |
| `3` | 批量标记已读 | ❌ 不参与聚类 |
| `-1` | 不喜欢 | ✅ 负向聚类 |

**核心公式：**
```
FinalScore = MaxSim(article, pos_centroids) - α * MaxSim(article, neg_centroids)
α = 1.5 (对不喜欢内容更敏感)
```

---

## 技术栈映射

| 功能 | Python | Rust |
|------|--------|------|
| 数据库 | `sqlite3` | `rusqlite` + `r2d2` (连接池) |
| 聚类 | `sklearn.cluster.KMeans` | `linfa-clustering` + `ndarray` |
| RSS 解析 | `feedparser` | `feed-rs` |
| HTTP 请求 | `requests` | `reqwest` + `tokio` |
| 向量运算 | `numpy` | `ndarray` |
| 前端 | `streamlit` | React (Vite) + Tauri v2 |

---

## 项目结构

```
ZenFlow/
├── python_prototype/          # 保留原 Python 代码作为参考
├── src-tauri/                 # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs            # Tauri 入口
│       ├── lib.rs             # 库导出
│       ├── config.rs          # 配置常量
│       ├── db/
│       │   ├── mod.rs
│       │   ├── schema.rs      # SQL schema
│       │   ├── pool.rs        # 连接池
│       │   └── operations.rs  # CRUD 操作
│       ├── feed/
│       │   ├── mod.rs
│       │   ├── fetcher.rs     # RSS 抓取
│       │   └── parser.rs      # 解析逻辑
│       ├── embedding/
│       │   ├── mod.rs
│       │   └── client.rs      # SiliconFlow API
│       ├── algorithm/
│       │   ├── mod.rs
│       │   ├── score.rs       # 计算推荐分数
│       │   └── cluster.rs     # K-Means 聚类
│       └── commands/
│           ├── mod.rs
│           └── article.rs     # Tauri Commands
├── src/                       # React 前端
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── ArticleList.tsx
│   │   ├── ArticleCard.tsx
│   │   └── FeedbackButtons.tsx
│   ├── hooks/
│   │   └── useArticles.ts
│   └── types/
│       └── article.ts
├── package.json
├── vite.config.ts
└── tsconfig.json
```

---

## 阶段一：项目初始化

### 1.1 创建 Tauri v2 项目
- [ ] 使用 `npm create tauri-app@latest` 初始化
- [ ] 配置 `src-tauri/tauri.conf.json`

### 1.2 配置 Cargo.toml 依赖
```toml
[dependencies]
# Tauri
tauri = { version = "2", features = ["shell-open"] }
tauri-plugin-shell = "2"

# 异步运行时
tokio = { version = "1", features = ["full"] }

# 数据库
rusqlite = { version = "0.32", features = ["bundled"] }
r2d2 = "0.8"
r2d2_sqlite = "0.25"

# 向量与聚类
ndarray = "0.16"
linfa = "0.7"
linfa-clustering = "0.7"
linfa-nn = "0.7"  # 可选，加速聚类

# RSS 解析
feed-rs = "2"

# HTTP 客户端
reqwest = { version = "0.12", features = ["json"] }

# 序列化
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# 工具
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## 阶段二：Rust 后端核心模块

### 2.1 数据库层 (`db/`)
- [ ] 定义 schema (articles, clusters 表)
- [ ] 实现连接池管理
- [ ] 实现 CRUD 操作：
  - `save_article()`
  - `update_status()`
  - `get_articles(status, limit)`
  - `get_vectors_by_status(statuses)`
  - `save_clusters()`
  - `load_clusters()`

### 2.2 RSS 抓取 (`feed/`)
- [ ] 使用 `feed-rs` 解析 RSS
- [ ] 使用 `reqwest` 异步抓取
- [ ] 支持 arXiv 特有字段解析

### 2.3 Embedding API (`embedding/`)
- [ ] 调用 SiliconFlow API
- [ ] 错误处理与重试
- [ ] 向量序列化/反序列化

### 2.4 算法层 (`algorithm/`)

**score.rs:**
```rust
pub fn compute_score(
    article_vector: &Array1<f32>,
    pos_centroids: &[Array1<f32>],
    neg_centroids: &[Array1<f32>],
    alpha: f32,  // 1.5
) -> f32 {
    // FinalScore = max(dot(article, pos)) - alpha * max(dot(article, neg))
}
```

**cluster.rs:**
```rust
pub fn update_clusters(
    pos_vectors: Vec<(Array1<f32>, f32)>,  // (vector, weight)
    neg_vectors: Vec<Array1<f32>>,
    max_clusters: usize,
) -> (Vec<Array1<f32>>, Vec<Array1<f32>>);  // (pos_centroids, neg_centroids)
```

### 2.5 Tauri Commands (`commands/`)
- [ ] `fetch_articles()` - 抓取新文章
- [ ] `get_articles(status, limit, offset)` - 获取文章列表
- [ ] `update_status(id, status)` - 更新状态
- [ ] `refresh_scores()` - 重新计算分数
- [ ] `mark_all_read()` - 批量标记已读 (status=3)

---

## 阶段三：React 前端

### 3.1 基础组件
- [ ] ArticleList - 文章列表
- [ ] ArticleCard - 单篇文章卡片
- [ ] FeedbackButtons - 反馈按钮 (点击/点赞/不喜欢/跳过)

### 3.2 状态管理
- [ ] 使用 React Query 或 SWR 管理 API 状态
- [ ] 实现 optimistic updates

### 3.3 UI 设计
- [ ] 按 score 排序展示
- [ ] 未读/已读筛选
- [ ] 批量操作

---

## 阶段四：集成与优化

- [ ] 端到端测试
- [ ] 性能优化 (大列表虚拟滚动)
- [ ] 错误边界处理
- [ ] 打包发布

---

## 当前进度

**正在进行：阶段一 - 项目初始化**