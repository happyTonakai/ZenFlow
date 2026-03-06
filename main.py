import logging
import sys

import uvicorn
from fastapi import FastAPI
from fastapi.responses import HTMLResponse
from pydantic import BaseModel

import algorithm
import config
import engine

logging.basicConfig(
    level=logging.INFO,
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
    status: int


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
        .tabs { display: flex; gap: 10px; margin-bottom: 20px; }
        .tab { padding: 10px 20px; background: #ddd; border: none; cursor: pointer; border-radius: 5px; }
        .tab.active { background: #4CAF50; color: white; }
        .article { background: white; padding: 15px; margin: 10px 0; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }
        .article h3 { margin: 0 0 10px 0; }
        .article .meta { color: #666; font-size: 14px; }
        .btn { padding: 8px 16px; margin-right: 5px; cursor: pointer; border: none; border-radius: 4px; }
        .btn-like { background: #4CAF50; color: white; }
        .btn-dislike { background: #f44336; color: white; }
        .btn-action { background: #2196F3; color: white; padding: 12px 24px; margin: 5px; cursor: pointer; border: none; border-radius: 5px; }
        .sidebar { background: white; padding: 20px; border-radius: 8px; margin-top: 20px; }
        .log { background: #1e1e1e; color: #0f0; padding: 15px; border-radius: 5px; font-family: monospace; font-size: 12px; max-height: 300px; overflow-y: auto; white-space: pre-wrap; }
        .stats { display: flex; gap: 20px; flex-wrap: wrap; }
        .stat-item { background: #f0f0f0; padding: 10px 15px; border-radius: 5px; }
    </style>
</head>
<body>
    <h1>📖 ZenFlow - AI Paper Recommendation</h1>
    
    <div class="tabs">
        <button class="tab active" onclick="showTab('recommend')">今日推荐</button>
        <button class="tab" onclick="showTab('all')">全部论文</button>
    </div>
    
    <div id="recommend">
        <h2>智能推荐</h2>
        <div id="recommend-list"></div>
    </div>
    
    <div id="all" style="display:none">
        <h2>全部论文</h2>
        <div id="all-list"></div>
    </div>
    
    <div class="sidebar">
        <h3>操作</h3>
        <button class="btn-action" onclick="fetchData()">🔄 刷新数据</button>
        <button class="btn-action" onclick="importPrefs()">⭐ 导入偏好论文</button>
        <button class="btn-action" onclick="calcScores()">📊 重新计算分数</button>
        <div id="log" class="log"></div>
        <h3>统计</h3>
        <div class="stats" id="stats"></div>
        <h3>聚类</h3>
        <div class="stats" id="clusters"></div>
    </div>
    
    <script>
        function log(msg) {
            const el = document.getElementById('log');
            const time = new Date().toLocaleTimeString();
            el.innerHTML += `[${time}] ${msg}\\n`;
            el.scrollTop = el.scrollHeight;
        }
        
        function showTab(name) {
            document.getElementById('recommend').style.display = name === 'recommend' ? 'block' : 'none';
            document.getElementById('all').style.display = name === 'all' ? 'block' : 'none';
            document.querySelectorAll('.tab').forEach((t, i) => {
                t.classList.toggle('active', (i === 0 && name === 'recommend') || (i === 1 && name === 'all'));
            });
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
            loadRecommend();
        }
        
        function renderArticle(a) {
            const score = a.score?.toFixed(2) || '0.00';
            const abs = a.translated_abstract || a.abstract || '';
            return `
                <div class="article">
                    <h3><a href="${a.link}" target="_blank">${a.title}</a></h3>
                    <div class="meta">评分: ${score} | 状态: ${a.status} | ${a.timestamp || ''}</div>
                    ${abs ? `<details><summary>查看摘要</summary><p>${abs.substring(0, 500)}...</p></details>` : ''}
                    <button class="btn btn-like" onclick="updateStatus('${a.id}', 2)">👍 喜欢</button>
                    <button class="btn btn-dislike" onclick="updateStatus('${a.id}', -1)">👎 不喜欢</button>
                </div>
            `;
        }
        
        async function loadRecommend() {
            const data = await api('GET', '/recommend?limit=20');
            document.getElementById('recommend-list').innerHTML = data?.map(renderArticle).join('') || '暂无推荐';
        }
        
        async function loadAll() {
            const data = await api('GET', '/articles?limit=100&order_by=timestamp DESC');
            document.getElementById('all-list').innerHTML = data?.map(renderArticle).join('') || '暂无';
        }
        
        async function fetchData() {
            log('📥 开始抓取 RSS...');
            const r = await api('POST', '/fetch');
            log('✅ 完成! 获取 ' + (r?.count || 0) + ' 篇文章');
            loadRecommend();
            loadStats();
        }
        
        async function importPrefs() {
            log('📥 开始导入偏好论文...');
            const r = await api('POST', '/preferences/import');
            log('✅ 导入完成! ' + (r?.count || 0) + ' 篇');
            loadRecommend();
            loadStats();
        }
        
        async function calcScores() {
            log('📊 计算分数...');
            const r = await api('POST', '/scores');
            log('✅ 完成');
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
        
        loadRecommend();
        loadAll();
        loadStats();
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
    engine.update_article_status(update.article_id, update.status)
    logger.info(f"📝 文章 {update.article_id} 状态更新为 {update.status}")
    return {"status": "ok"}


@app.post("/fetch")
def fetch_feeds():
    logger.info("📥 开始抓取 RSS...")
    articles = engine.fetch_feeds()
    logger.info(f"📰 获取到 {len(articles)} 篇文章")

    for i, art in enumerate(articles):
        text = f"{art['title']} {art['abstract']}"
        vector = engine.embed_text(text)
        engine.save_article(art, vector)
        if (i + 1) % 10 == 0:
            logger.info(f"  已处理 {i + 1}/{len(articles)}")

    logger.info("🎯 更新聚类...")
    algorithm.update_clusters()

    logger.info("📊 计算分数...")
    algorithm.calculate_all_scores()

    return {"status": "ok", "count": len(articles)}


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
    logger.info("✅ 分数计算完成")
    return {"status": "ok"}


@app.get("/stats")
def get_stats():
    return engine.get_article_count_by_status()


@app.get("/clusters")
def get_clusters():
    pos = engine.load_clusters("positive")
    neg = engine.load_clusters("negative")
    return {"positive": len(pos), "negative": len(neg)}


@app.get("/recommend")
def get_recommend(limit: int = 20):
    return algorithm.get_diverse_recommendations(limit=limit)


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
            logger.info(f"  ✅ 已保存并标记为喜欢")

        logger.info("✅ 偏好论文导入完成")
        return {"status": "ok", "count": len(papers)}
    except Exception as e:
        logger.error(f"导入失败: {e}")
        return {"status": "error", "detail": str(e)}


if __name__ == "__main__":
    uvicorn.run(app, host="0.0.0.0", port=8052)
