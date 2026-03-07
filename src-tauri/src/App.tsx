import { useState, useEffect } from "react";
import { ArticleList } from "./components/ArticleList";
import { 
  useArticles, 
  useStats, 
  fetchNewArticles, 
  markAllRead, 
  refreshRecommendations,
  checkInitialized 
} from "./hooks/useArticles";
import { ArticleStatus } from "./types/article";
import "./App.css";

type FilterType = 'unread' | 'liked' | 'all';

function App() {
  const [filter, setFilter] = useState<FilterType>('unread');
  const [isInitialized, setIsInitialized] = useState(false);
  const [fetching, setFetching] = useState(false);

  const statusFilter = filter === 'unread' ? ArticleStatus.UNREAD 
    : filter === 'liked' ? ArticleStatus.LIKED 
    : null;

  const { articles, loading, error, updateArticleStatusLocal } = useArticles(statusFilter);
  const { stats, refetch: refetchStats } = useStats();

  useEffect(() => {
    checkInitialized().then(setIsInitialized);
  }, []);

  // 处理文章状态变化（就地更新）
  const handleStatusChange = (articleId: string, newStatus: number) => {
    // 就地更新文章状态
    updateArticleStatusLocal(articleId, newStatus);
    // 刷新统计数据
    refetchStats();
    // 检查是否初始化完成
    checkInitialized().then(setIsInitialized);
  };

  const handleFetchArticles = async () => {
    setFetching(true);
    try {
      const count = await fetchNewArticles();
      console.log(`Fetched ${count} new articles`);
    } catch (e) {
      console.error('Fetch failed:', e);
    } finally {
      setFetching(false);
    }
  };

  const handleMarkAllRead = async () => {
    try {
      const count = await markAllRead();
      console.log(`Marked ${count} articles as read`);
      // 刷新统计
      refetchStats();
    } catch (e) {
      console.error('Mark all read failed:', e);
    }
  };

  const handleRefreshRecommendations = async () => {
    try {
      const result = await refreshRecommendations();
      console.log('Refresh result:', result);
    } catch (e) {
      console.error('Refresh failed:', e);
    }
  };

  return (
    <div className="app">
      {/* Header */}
      <header className="header">
        <h1>ZenFlow</h1>
        <p className="subtitle">AI Paper & News Recommendation</p>
      </header>

      {/* Stats Bar */}
      <div className="stats-bar">
        <div className="stat-item">
          <span className="stat-value">{stats?.unread || 0}</span>
          <span className="stat-label">未读</span>
        </div>
        <div className="stat-item">
          <span className="stat-value">{stats?.liked || 0}</span>
          <span className="stat-label">喜欢</span>
        </div>
        <div className="stat-item">
          <span className="stat-value">{stats?.clicked || 0}</span>
          <span className="stat-label">点击</span>
        </div>
        <div className="stat-item initialized">
          <span className="stat-value">{isInitialized ? '✓' : '○'}</span>
          <span className="stat-label">已初始化</span>
        </div>
      </div>

      {/* Toolbar */}
      <div className="toolbar">
        <div className="filter-tabs">
          <button 
            className={filter === 'unread' ? 'active' : ''}
            onClick={() => setFilter('unread')}
          >
            未读
          </button>
          <button 
            className={filter === 'liked' ? 'active' : ''}
            onClick={() => setFilter('liked')}
          >
            喜欢
          </button>
          <button 
            className={filter === 'all' ? 'active' : ''}
            onClick={() => setFilter('all')}
          >
            全部
          </button>
        </div>

        <div className="actions">
          <button onClick={handleFetchArticles} disabled={fetching}>
            {fetching ? '抓取中...' : '📥 抓取新文章'}
          </button>
          <button onClick={handleMarkAllRead}>
            ✓ 全部已读
          </button>
          <button onClick={handleRefreshRecommendations}>
            🔄 刷新推荐
          </button>
        </div>
      </div>

      {/* Article List */}
      <main className="main">
        <ArticleList 
          articles={articles}
          loading={loading}
          error={error}
          onStatusChange={handleStatusChange}
          currentTab={statusFilter}
        />
      </main>
    </div>
  );
}

export default App;
