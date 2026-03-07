# ZenFlow Tauri v2 开发计划

## 项目概述

AI 驱动的学术论文推荐系统。从 Python 原型迁移至 Tauri v2 + Rust 本地原生应用。

---

## 状态机制设计

| status | 含义 | 聚类参与 |
|--------|------|----------|
| `0` | 未读 | ❌ |
| `1` | 已点击（真正阅读） | ✅ 正向聚类，权重 1.0 |
| `2` | 已点赞 | ✅ 正向聚类，权重 2.0 |
| `3` | 批量标记已读/跳过 | ❌ 不参与聚类 |
| `-1` | 不喜欢 | ✅ 负向聚类 |

**核心公式：**
```
FinalScore = MaxSim(article, pos_centroids) - α * MaxSim(article, neg_centroids)
α = 1.5 (对不喜欢内容更敏感)
```

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
│       ├── settings.rs        # 用户设置管理（含密钥链存储）
│       ├── db/
│       │   ├── mod.rs
│       │   ├── schema.rs      # SQL schema（articles, clusters, settings）
│       │   ├── pool.rs        # 连接池
│       │   └── operations.rs  # CRUD 操作
│       ├── feed/
│       │   ├── mod.rs
│       │   └── fetcher.rs     # RSS 抓取（开发模式读取本地文件）
│       ├── embedding/
│       │   ├── mod.rs
│       │   └── client.rs      # SiliconFlow Embedding API
│       ├── algorithm/
│       │   ├── mod.rs
│       │   ├── score.rs       # 推荐分数计算
│       │   └── cluster.rs     # K-Means 聚类
│       └── commands/
│           ├── mod.rs
│           ├── article.rs     # 文章相关 Commands
│           └── init.rs        # 初始化向导 Commands
├── src/                       # React 前端
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── ArticleList.tsx
│   │   ├── ArticleCard.tsx    # 含点赞/点踩/跳过按钮
│   │   └── WelcomeWizard.tsx  # 初始化向导界面
│   ├── hooks/
│   │   └── useArticles.ts
│   └── types/
│       └── article.ts
├── package.json
├── vite.config.ts
└── tsconfig.json
```

---

## 已完成 ✅

### 1. 项目基础架构
- [x] Tauri v2 项目初始化
- [x] React + Vite 前端配置
- [x] 数据库 Schema（articles, clusters, settings 表）
- [x] 数据库连接池

### 2. Rust 后端核心模块
- [x] **数据库层** (`db/`)
  - SQLite 单文件数据库（`~/.zenflow/zenflow.db`）
  - 完整的 CRUD 操作
  - 设置表支持

- [x] **RSS 抓取** (`feed/`)
  - 开发模式：读取本地 `test_rss.xml`
  - arXiv RSS 解析
  - arXiv API 获取单篇论文

- [x] **Embedding API** (`embedding/`)
  - SiliconFlow API 调用
  - 支持自定义 API Key
  - 自动从设置读取 API Key

- [x] **算法层** (`algorithm/`)
  - K-Means 聚类（`linfa-clustering`）
  - 余弦相似度计算
  - 推荐分数计算（支持自定义 α）

- [x] **用户设置** (`settings.rs`)
  - 系统密钥链存储 API Key（macOS/Windows/Linux）
  - 其他设置存 SQLite
  - 全局缓存

### 3. Tauri Commands
- [x] `fetch_articles()` - 抓取新文章
- [x] `get_articles(status, limit, offset)` - 获取文章列表
- [x] `update_status(id, status)` - 更新状态
- [x] `mark_all_read()` - 批量标记已读
- [x] `refresh_recommendations()` - 重新计算聚类和分数
- [x] `initialize_app()` - 初始化向导
- [x] `get_settings()` / `save_settings()` - 设置管理
- [x] `translate_text()` - SiliconFlow LLM 翻译

### 4. React 前端
- [x] **ArticleCard** - 文章卡片（标题、摘要、操作按钮）
  - 按钮始终显示，当前状态高亮
  - 点击激活状态可取消
  - 点击标题打开链接，不覆盖点赞状态

- [x] **ArticleList** - 文章列表
  - 标签页：未读 / 喜欢 / 全部
  - 标签页感知的状态更新逻辑

- [x] **WelcomeWizard** - 初始化向导
  - 4 步流程：选择分类 → 添加论文 → 配置参数 → 完成
  - Tag 输入交互（回车添加分类）
  - 分类推荐快速选择
  - API Key 密码输入（眼睛图标切换）
  - 参数滑块（α、多样性比例）
  - 翻译开关

- [x] **状态管理**
  - `useArticles` hook
  - `useStats` hook
  - 标签页感知的本地状态更新

---

## 待完成 ⏳

### 阶段一：推荐展示优化
- [ ] **混合推荐策略**
  - 70% 高相似度 + 30% 随机探索
  - 按用户配置的 `diversity_ratio` 调整
  
- [ ] **每日自动刷新**
  - 定时任务抓取新 RSS
  - 自动为新文章生成 embedding
  - 重新计算推荐分数

- [ ] **推荐分数可视化**
  - 在 UI 显示每篇文章的推荐分数
  - 显示与聚类的相似度

### 阶段二：生产环境 RSS
- [ ] **网络 RSS 抓取**
  - 从 `settings.get_rss_feeds()` 获取订阅列表
  - 异步并发抓取多个分类
  - 错误处理和重试

- [ ] **增量更新**
  - 只抓取新发布的文章
  - 基于文章 ID 去重
  - 增量生成 embedding

### 阶段三：增强功能
- [ ] **翻译功能完善**
  - 摘要自动翻译（使用 Qwen API）
  - 翻译结果缓存
  - 显示/隐藏翻译切换

- [ ] **搜索功能**
  - 本地标题/摘要搜索
  - 基于向量的语义搜索

- [ ] **数据导出**
  - 导出喜欢的论文列表
  - 导出聚类中心（用于迁移）

### 阶段四：性能与体验
- [ ] **大列表虚拟滚动**
  - 当文章数量 > 1000 时优化性能

- [ ] **离线支持**
  - 无网络时显示已缓存文章
  - 延迟同步机制

- [ ] **错误处理**
  - API 限流处理
  - 网络错误重试
  - 用户友好的错误提示

---

## 配置项说明

| 配置项 | 默认值 | 说明 |
|--------|--------|------|
| `arxiv_categories` | `["cs.AI", "cs.LG", "cs.CL"]` | arXiv 分类列表 |
| `siliconflow_api_key` | - | 存储在系统密钥链 |
| `pos_clusters` | 5 | 正向聚类数量 |
| `neg_clusters` | 3 | 负向聚类数量 |
| `daily_papers` | 20 | 每日展示论文数 |
| `negative_alpha` | 1.5 | 负向惩罚系数 |
| `diversity_ratio` | 0.3 | 随机探索比例 |
| `enable_translation` | true | 是否启用翻译 |
| `translation_model` | `Qwen2.5-7B` | 翻译模型 |

---

## 本地开发

```bash
# 进入项目目录
cd src-tauri

# 安装依赖
npm install

# 开发模式（热更新）
npm run tauri dev

# 构建生产版本
npm run tauri build
```

---

## 最近更新

- **2025-03-07**: 实现初始化向导界面
- **2025-03-07**: API Key 改用系统密钥链存储
- **2025-03-07**: 完善文章状态交互逻辑（标签页感知）
- **2025-03-07**: Tauri v2 + Rust 架构迁移完成
