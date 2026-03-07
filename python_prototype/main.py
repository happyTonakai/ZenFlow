import logging
import sys
from typing import Optional

import numpy as np
import uvicorn
from fastapi import FastAPI
from fastapi.responses import HTMLResponse
from pydantic import BaseModel

import algorithm
import config
import engine

logging.basicConfig(
    level=logging.DEBUG if config.DEBUG else logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    stream=sys.stdout,
)
logger = logging.getLogger(__name__)

app = FastAPI(title="ZenFlow")


class ArticleIn(BaseModel):
    title: str
    link: str
    abstract: str
    source: str = "arxiv"


class StatusUpdate(BaseModel):
    article_id: str
    status: Optional[int] = None


@app.on_event("startup")
def startup():
    logger.info("🚀 启动 ZenFlow...")
    engine.init_db()
    logger.info("✅ 数据库就绪")


@app.get("/", response_class=HTMLResponse)
def root():
    return """
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>ZenFlow - AI Paper Recommendation</title>
    <style>
        * { box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 900px; margin: 0 auto; padding: 20px; background: #f5f5f5; }
        h1 { color: #333; }
        .article { background: white; padding: 15px; margin: 10px 0; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .article h3 { margin: 0 0 10px 0; }
        .article .meta { color: #666; font-size: 14px; }
        .article .abstract { color: #444; font-size: 14px; line-height: 1.6; margin: 10px 0; }
        .btn { padding: 8px 16px; margin-right: 5px; cursor: pointer; border: none; border-radius: 4px; }
        .btn-like { background: #4CAF50; color: white; }
        .btn-dislike { background: #f44336; color: white; }
        .btn-action { background: #2196F3; color: white; padding: 12px 24px; margin: 5px; cursor: pointer; border: none; border-radius: 5px; }
        .btn-primary { background: #FF9800; color: white; font-size: 18px; padding: 15px 30px; }
        .sidebar { background: white; padding: 20px; border-radius: 8px; margin-top: 20px; }
        .log { background: #1e1e1e; color: #0f0; padding: 15px; border-radius: 5px; font-family: monospace; font-size: 12px; max-height: 300px; overflow-y: auto; white-space: pre-wrap; }
        .stats { display: flex; gap: 20px; flex-wrap: wrap; }
        .stat-item { background: #f0f0f0; padding: 10px 15px; border-radius: 5px; }
        .init-panel { background: white; padding: 30px; border-radius: 12px; box-shadow: 0 4px 12px rgba(0,0,0,0.15); text-align: center; margin: 40px auto; max-width: 600px; }
        .init-panel h2 { color: #FF9800; }
        .divider { display: flex; align-items: center; margin: 30px 0 20px 0; }
        .divider::before, .divider::after { content: ''; flex: 1; border-bottom: 2px dashed #ccc; }
        .divider span { padding: 0 15px; color: #888; font-size: 14px; background: #f5f5f5; }
        .init-panel p { color: #666; line-height: 1.8; }
        .progress-bar { background: #e0e0e0; border-radius: 10px; height: 20px; margin: 20px 0; overflow: hidden; }
        .progress-fill { background: #4CAF50; height: 100%; transition: width 0.3s; }
        .hidden { display: none; }
    </style>
</head>
<body>
    <h1>📖 ZenFlow - AI Paper Recommendation</h1>

    <div id="init-panel" class="init-panel hidden">
        <h2>🎯 初始化系统</h2>
        <p>在开始使用之前，请导入您感兴趣的论文来建立偏好模型。</p>
        <p>系统需要至少 <strong id="required-count">5</strong> 篇偏好论文才能开始智能推荐。</p>
        <div class="progress-bar">
            <div id="progress-fill" class="progress-fill" style="width: 0%"></div>
        </div>
        <p id="progress-text">当前已导入: <strong id="current-count">0</strong> 篇</p>
        <button class="btn-action btn-primary" onclick="importPrefs()">⭐ 导入偏好论文</button>
        <p style="font-size: 12px; color: #999; margin-top: 20px;">提示: 请在 preferences.txt 文件中添加您喜欢的论文链接</p>
    </div>

    <div id="main-panel" class="hidden">
        <div id="recommend">
            <h2>智能推荐</h2>
            <p style="color:#888; font-size:13px; margin-bottom:10px;">前 70% 按匹配度排序 | 后 30% 多样性探索</p>
            <div id="recommend-list"></div>
            <button class="btn-action" onclick="loadMore()">➕ 加载更多</button>
        </div>

        <div class="sidebar">
            <h3>操作</h3>
            <button class="btn-action" onclick="fetchData()">🔄 刷新数据</button>
            <button class="btn-action" onclick="importPrefs()">⭐ 导入偏好论文</button>
            <button class="btn-action" onclick="calcScores()">📊 重新计算分数</button>
            <button class="btn-action" onclick="updateVotes()">🔄 更新投票数</button>
            <div id="log" class="log"></div>
            <h3>统计</h3>
            <div class="stats" id="stats"></div>
            <h3>聚类</h3>
            <div class="stats" id="clusters"></div>
            <h3>调试</h3>
            <button class="btn-action" onclick="showDebugPanel()">🔍 相似度分析</button>
            <div id="debug-panel" style="display:none; margin-top:10px;">
                <select id="debug-article-select" style="width:100%; padding:8px; margin-bottom:10px;"></select>
                <div id="debug-result" style="background:#f5f5f5; padding:10px; border-radius:5px; font-size:12px;"></div>
            </div>
        </div>
    </div>

    <script>
        let shownIds = [];
        let isInitialized = false;

        function log(msg) {
            const el = document.getElementById('log');
            const time = new Date().toLocaleTimeString();
            el.innerHTML += `[${time}] ${msg}\\n`;
            el.scrollTop = el.scrollHeight;
        }

        async function api(method, url, body = null) {
            try {
                const opts = { method, headers: {'Content-Type': 'application/json'} };
                if (body) opts.body = JSON.stringify(body);
                const r = await fetch(url, opts);
                return await r.json();
            } catch(e) { log('API错误: ' + e); return null; }
        }

        async function updateStatus(id, status) {
            await api('POST', '/status', {article_id: id, status});
            const el = document.querySelector(`[data-id="${id}"]`);
            if (el) el.style.opacity = '0.5';
        }

        function renderArticle(a) {
            const score = a.score?.toFixed(2) || '0.00';
            const abs = a.translated_abstract || a.abstract || '';

            // 从数据库数据构建投票显示
            let votesHtml = '';
            if (a.hf_upvotes !== null && a.hf_upvotes !== undefined) {
                votesHtml += `<span title="HuggingFace">🤗 ${a.hf_upvotes}</span> `;
            }
            if (a.ax_upvotes !== null && a.ax_downvotes !== null) {
                const net = (a.ax_upvotes || 0) - (a.ax_downvotes || 0);
                votesHtml += `<span title="AlphaXiv">🔬 ${net >= 0 ? '+' : ''}${net}</span>`;
            }
            if (votesHtml) {
                votesHtml = '| ' + votesHtml;
            }

            return `
                <div class="article" data-id="${a.id}">
                    <h3><a href="${a.link}" target="_blank" onclick="trackClick('${a.id}')">${a.title}</a></h3>
                    <div class="meta">
                        评分: ${score} | 状态: ${a.status} | ${a.timestamp || ''}
                        <span class="votes">${votesHtml}</span>
                    </div>
                    ${abs ? `<p class="abstract">${abs}</p>` : ''}
                    <button class="btn btn-like" onclick="updateStatus('${a.id}', 2)">👍 喜欢</button>
                    <button class="btn btn-dislike" onclick="updateStatus('${a.id}', -1)">👎 不喜欢</button>
                </div>
            `;
        }

        async function trackClick(id) {
            await api('POST', '/status', {article_id: id, status: 1});
        }

        async function translateArticle(id) {
            return await api('POST', '/translate', {article_id: id});
        }

        async function loadRecommend() {
            const exclude = shownIds.join(',');
            const data = await api('GET', '/recommend?limit=20&exclude_ids=' + exclude);

            let html = '';

            if (data?.score_based?.length > 0) {
                html += data.score_based.map(renderArticle).join('');
            }

            if (data?.diverse?.length > 0) {
                html += '<div class="divider"><span>🎲 多样性推荐</span></div>';
                html += data.diverse.map(renderArticle).join('');
            }

            if (!html) {
                html = '暂无推荐';
            }

            if (exclude) {
                document.getElementById('recommend-list').innerHTML += html;
            } else {
                document.getElementById('recommend-list').innerHTML = html;
            }

            // 处理翻译
            const allArticles = [...(data?.score_based || []), ...(data?.diverse || [])];
            allArticles.forEach(a => {
                if (!shownIds.includes(a.id)) {
                    shownIds.push(a.id);
                    // 翻译摘要
                    if (!a.translated_abstract) {
                        translateArticle(a.id).then(r => {
                            if (r?.translated) {
                                const el = document.querySelector(`[data-id="${a.id}"] .abstract`);
                                if (el) el.textContent = r.translated;
                            }
                        });
                    }
                }
            });
        }

        async function loadMore() {
            await loadRecommend();
        }

        async function fetchData() {
            log('📥 开始抓取 RSS...');
            shownIds = [];
            const r = await api('POST', '/fetch');
            log('✅ 完成! 获取 ' + (r?.count || 0) + ' 篇文章');
            loadRecommend();
            loadStats();
        }

        async function importPrefs() {
            log('📥 开始导入偏好论文...');
            const r = await api('POST', '/preferences/import');
            log('✅ 导入完成! ' + (r?.count || 0) + ' 篇');
            await checkInitStatus();
            if (isInitialized) {
                loadRecommend();
                loadStats();
            }
        }

        async function calcScores() {
            log('📊 计算分数...');
            const r = await api('POST', '/scores');
            log('✅ 完成');
            loadRecommend();
        }

        async function updateVotes() {
            log('🔄 更新投票数据...');
            const r = await api('POST', '/votes/update');
            log(`✅ 完成: ${r?.updated || 0} 成功, ${r?.failed || 0} 失败`);
            loadRecommend();
        }

        async function loadStats() {
            const s = await api('GET', '/stats');
            document.getElementById('stats').innerHTML = `
                <div class="stat-item">未读: ${s?.[0]||0}</div>
                <div class="stat-item">已点击: ${s?.[1]||0}</div>
                <div class="stat-item">点赞: ${s?.[2]||0}</div>
                <div class="stat-item">点踩: ${s?.[-1]||0}</div>
            `;
            const c = await api('GET', '/clusters');
            document.getElementById('clusters').innerHTML = `
                <div class="stat-item">正向: ${c?.positive||0}</div>
                <div class="stat-item">负向: ${c?.negative||0}</div>
            `;
        }

        async function checkInitStatus() {
            const status = await api('GET', '/init-status');
            isInitialized = status?.is_initialized || false;
            const currentCount = status?.liked_count || 0;
            const requiredCount = status?.required_count || 5;

            document.getElementById('current-count').textContent = currentCount;
            document.getElementById('required-count').textContent = requiredCount;

            const progress = Math.min(100, (currentCount / requiredCount) * 100);
            document.getElementById('progress-fill').style.width = progress + '%';

            if (isInitialized) {
                document.getElementById('init-panel').classList.add('hidden');
                document.getElementById('main-panel').classList.remove('hidden');
            } else {
                document.getElementById('init-panel').classList.remove('hidden');
                document.getElementById('main-panel').classList.add('hidden');
            }
        }

        // 启动时检查初始化状态
        checkInitStatus();

        // 调试功能
        async function showDebugPanel() {
            const panel = document.getElementById('debug-panel');
            if (panel.style.display === 'none') {
                panel.style.display = 'block';
                // 加载文章列表
                const articles = await api('GET', '/debug/articles');
                const select = document.getElementById('debug-article-select');
                select.innerHTML = articles?.map(a =>
                    `<option value="${a.id}">[${a.status===2?'👍':a.status===-1?'👎':a.status===1?'👁':'📄'}] ${a.title} (score: ${a.score?.toFixed(2)})</option>`
                ).join('') || '';
            } else {
                panel.style.display = 'none';
            }
        }

        document.getElementById('debug-article-select')?.addEventListener('change', async (e) => {
            const articleId = e.target.value;
            if (!articleId) return;
            const result = await api('GET', '/debug/similarity/' + articleId);
            const el = document.getElementById('debug-result');
            if (result?.error) {
                el.innerHTML = `<span style="color:red;">${result.error}</span>`;
                return;
            }
            el.innerHTML = `
                <strong>${result.title?.substring(0, 50)}...</strong><br>
                <hr>
                <strong>正向相似度:</strong><br>
                ${result.positive_similarities?.map(s => `  聚类${s.cluster_id}: ${s.similarity}`).join('<br>') || '无'}<br>
                <strong>最大正向:</strong> ${result.max_positive_sim}<br>
                <hr>
                <strong>负向相似度:</strong><br>
                ${result.negative_similarities?.map(s => `  聚类${s.cluster_id}: ${s.similarity}`).join('<br>') || '无'}<br>
                <strong>最大负向:</strong> ${result.max_negative_sim}<br>
                <hr>
                <strong>公式:</strong> ${result.formula}<br>
                <strong style="color: #2196F3;">最终分数: ${result.final_score}</strong>
            `;
        });
    </script>
</body>
</html>
    """


@app.get("/articles")
def get_articles(
    status: int | None = None, limit: int = 50, order_by: str = "score DESC"
):
    return engine.get_articles(status=status, limit=limit, order_by=order_by)


@app.post("/articles")
def add_article(article: ArticleIn):
    logger.info(f"📥 添加文章: {article.title[:50]}...")
    text = f"{article.title} {article.abstract}"
    vector = engine.embed_text(text)
    article_dict = article.model_dump()
    engine.save_article(article_dict, vector)
    logger.info(f"✅ 已保存: {article.title[:30]}...")
    return {"status": "ok"}


@app.post("/status")
def update_status(update: StatusUpdate):
    if update.status is not None:
        engine.update_article_status(update.article_id, update.status)
        logger.info(f"📝 文章 {update.article_id} 状态更新为 {update.status}")
    return {"status": "ok"}


@app.post("/translate")
def translate_article(update: StatusUpdate):
    article = engine.get_article(update.article_id)
    if not article:
        return {"status": "error", "detail": "Article not found"}
    result = engine.ensure_translated(article)
    logger.info(f"📝 文章 {update.article_id} 翻译完成")
    return {"status": "ok", "translated": result.get("translated_abstract")}


@app.post("/fetch")
def fetch_feeds():
    logger.info("📥 开始抓取 RSS...")
    articles = engine.fetch_feeds()
    logger.info(f"📰 获取到 {len(articles)} 篇文章")

    # 先检查哪些文章已存在
    all_ids = [a["id"] for a in articles]
    existing_ids = engine.get_existing_article_ids(all_ids)

    # 过滤出新文章
    new_articles = [a for a in articles if a["id"] not in existing_ids]
    logger.info(f"🆕 其中 {len(new_articles)} 篇为新文章，{len(existing_ids)} 篇已存在")

    if not new_articles:
        logger.info("✅ 没有新文章需要处理")
        return {"status": "ok", "count": 0, "new": 0, "existing": len(existing_ids)}

    max_articles = config.DEBUG_MAX_ARTICLES if config.DEBUG else len(new_articles)
    articles_to_process = new_articles[:max_articles]
    logger.info(
        f"{'🔧 DEBUG模式: ' if config.DEBUG else ''}将处理 {len(articles_to_process)} 篇新文章"
    )

    for i, art in enumerate(articles_to_process):
        text = f"{art['title']} {art['abstract']}"
        vector = engine.embed_text(text)
        engine.save_article(art, vector)
        if (i + 1) % 10 == 0:
            logger.info(f"  已处理 {i + 1}/{len(articles_to_process)}")

    logger.info("🎯 更新聚类...")
    algorithm.update_clusters()

    logger.info("📊 计算分数...")
    algorithm.calculate_all_scores()

    logger.info("📦 加载到内存...")
    engine.load_today_articles()

    return {
        "status": "ok",
        "count": len(articles_to_process),
        "new": len(new_articles),
        "existing": len(existing_ids),
    }


@app.post("/clusters")
def update_clusters():
    logger.info("🎯 开始更新聚类...")
    algorithm.update_clusters()
    logger.info("✅ 聚类更新完成")
    return {"status": "ok"}


@app.post("/scores")
def calculate_scores():
    logger.info("📊 开始计算分数...")
    algorithm.calculate_all_scores()
    logger.info("📦 更新内存...")
    engine.load_today_articles()
    logger.info("✅ 分数计算完成")
    return {"status": "ok"}


@app.get("/stats")
def get_stats():
    return engine.get_article_count_by_status()


@app.get("/votes/{article_id}")
def get_votes(article_id: str):
    """获取文章在 AlphaXiv 和 HuggingFace 上的社区投票数据"""
    return engine.get_community_votes(article_id)


@app.post("/votes/update")
def update_votes():
    """批量更新展示文章的投票数据"""
    logger.info("📊 开始更新投票数据...")
    # 获取前 50 篇未读文章
    articles = engine.get_articles(status=0, limit=50)
    article_ids = [a["id"] for a in articles]
    result = engine.update_votes_for_articles(article_ids)
    logger.info(f"✅ 更新完成: {result['updated']} 成功, {result['failed']} 失败")

    # 重新加载到内存
    engine.load_today_articles()

    return result


@app.get("/init-status")
def get_init_status():
    """检查系统是否已完成初始化(导入足够的偏好文章)"""
    liked_count = engine.get_liked_count()
    is_ready = engine.is_initialized()
    return {
        "is_initialized": is_ready,
        "liked_count": liked_count,
        "required_count": config.CLUSTER_TRIGGER_THRESHOLD,
    }


@app.get("/clusters")
def get_clusters():
    pos = engine.load_clusters("positive")
    neg = engine.load_clusters("negative")
    return {"positive": len(pos), "negative": len(neg)}


@app.get("/recommend")
def get_recommend(limit: int = 20, exclude_ids: str = ""):
    exclude_set = set(exclude_ids.split(",")) if exclude_ids else set()
    articles = engine.get_cached_articles()
    unread = [a for a in articles if a["status"] == 0 and a["id"] not in exclude_set]
    return algorithm.get_diverse_recommendations_from_list(unread, limit=limit)


@app.post("/preferences/import")
def import_preferences():
    logger.info("📥 开始导入偏好论文...")
    try:
        with open("preferences.txt", "r") as f:
            links = [line.strip() for line in f if line.strip()]
        arxiv_ids = [link.split("/abs/")[-1] for link in links if "/abs/" in link]
        logger.info(f"📋 找到 {len(arxiv_ids)} 个arxiv ID: {arxiv_ids}")

        papers = engine.fetch_arxiv_by_ids(arxiv_ids)
        logger.info(f"📄 获取到 {len(papers)} 篇论文")

        for i, paper in enumerate(papers):
            logger.info(f"  [{i + 1}/{len(papers)}] 处理: {paper['title'][:50]}...")
            text = f"{paper['title']} {paper['abstract']}"
            vector = engine.embed_text(text)
            if vector is None:
                logger.error(f"  ❌ Embedding 失败: {paper['title'][:30]}")
                continue
            engine.save_article(paper, vector)
            engine.update_article_status(paper["id"], 2)
            logger.info("  ✅ 已保存并标记为喜欢")

        logger.info("🎯 更新聚类...")
        algorithm.update_clusters()

        logger.info("📊 计算分数...")
        algorithm.calculate_all_scores()

        logger.info("📦 更新内存...")
        engine.load_today_articles()

        logger.info("✅ 偏好论文导入完成")
        return {"status": "ok", "count": len(papers)}
    except Exception as e:
        logger.error(f"导入失败: {e}")
        return {"status": "error", "detail": str(e)}


@app.get("/debug/similarity/{article_id}")
def debug_similarity(article_id: str):
    """调试接口：查看文章与聚类中心的相似度"""
    article = engine.get_article(article_id)
    if not article:
        return {"error": "Article not found"}

    if not article.get("vector"):
        return {"error": "Article has no vector"}

    vec = np.frombuffer(article["vector"], dtype=np.float32)

    pos_centroids = engine.load_clusters("positive")
    neg_centroids = engine.load_clusters("negative")

    # 计算与每个正向中心的相似度
    pos_sims = []
    for i, c in enumerate(pos_centroids):
        sim = float(np.dot(vec, c))
        pos_sims.append({"cluster_id": i, "similarity": round(sim, 4)})

    # 计算与每个负向中心的相似度
    neg_sims = []
    for i, c in enumerate(neg_centroids):
        sim = float(np.dot(vec, c))
        neg_sims.append({"cluster_id": i, "similarity": round(sim, 4)})

    # 计算最终分数 (使用 α 系数)
    p_sim = max(s["similarity"] for s in pos_sims) if pos_sims else 0.0
    n_sim = max(s["similarity"] for s in neg_sims) if neg_sims else 0.0
    alpha = config.NEGATIVE_PENALTY_ALPHA
    final_score = p_sim - alpha * n_sim

    return {
        "article_id": article_id,
        "title": article["title"],
        "status": article["status"],
        "positive_similarities": pos_sims,
        "negative_similarities": neg_sims,
        "max_positive_sim": round(p_sim, 4),
        "max_negative_sim": round(n_sim, 4),
        "alpha": alpha,
        "formula": f"{p_sim:.4f} - {alpha} × {n_sim:.4f} = {final_score:.4f}",
        "final_score": round(final_score, 4),
    }


@app.get("/debug/articles")
def debug_articles(status: int | None = None):
    """调试接口：列出所有文章 ID 和标题"""
    articles = engine.get_articles(status=status, limit=100)
    return [
        {
            "id": a["id"],
            "title": a["title"][:60] + "..." if len(a["title"]) > 60 else a["title"],
            "status": a["status"],
            "score": a.get("score", 0),
        }
        for a in articles
    ]


if __name__ == "__main__":
    engine.init_db()
    try:
        engine.load_today_articles()
    except Exception as e:
        logger.warning(f"加载文章失败(数据库可能为空): {e}")
    uvicorn.run(app, host="0.0.0.0", port=8052)
