import { useState, useEffect } from 'react';
import { Article } from '../types/article';
import { ArticleCard } from './ArticleCard';
import { getRecommendationDates, getArticlesByRecommendDate } from '../hooks/useArticles';

interface DateGroupedArticleListProps {
  onStatusChange: (articleId: string, newStatus: number) => void;
  currentTab: number | null;
}

export function DateGroupedArticleList({ onStatusChange, currentTab }: DateGroupedArticleListProps) {
  const [dates, setDates] = useState<string[]>([]);
  const [selectedDate, setSelectedDate] = useState<string | null>(null);
  const [articles, setArticles] = useState<Article[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setLoading(true);
    getRecommendationDates()
      .then((d) => {
        setDates(d);
        if (d.length > 0 && !selectedDate) {
          setSelectedDate(d[0]);
        }
      })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    if (!selectedDate) return;
    setLoading(true);
    getArticlesByRecommendDate(selectedDate)
      .then(setArticles)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [selectedDate]);

  // Handle local status updates
  const handleStatusChange = (articleId: string, newStatus: number) => {
    setArticles((prev) =>
      prev.map((a) => (a.id === articleId ? { ...a, status: newStatus } : a))
    );
    onStatusChange(articleId, newStatus);
  };

  if (loading && dates.length === 0) {
    return <div className="loading">加载中...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <p>加载失败: {error}</p>
      </div>
    );
  }

  if (dates.length === 0) {
    return (
      <div className="empty">
        <p>暂无每日推荐</p>
        <p style={{ fontSize: '14px', marginTop: '8px' }}>
          点击「生成今日推荐」按钮创建推荐批次
        </p>
      </div>
    );
  }

  return (
    <div>
      {/* Date tabs */}
      <div className="date-tabs" style={{
        display: 'flex',
        gap: '8px',
        marginBottom: '16px',
        flexWrap: 'wrap',
      }}>
        {dates.map((date) => (
          <button
            key={date}
            className={selectedDate === date ? 'active' : ''}
            onClick={() => setSelectedDate(date)}
            style={{
              padding: '6px 14px',
              borderRadius: '16px',
              border: '1px solid var(--border)',
              background: selectedDate === date ? 'var(--primary)' : 'var(--bg-secondary)',
              color: selectedDate === date ? 'white' : 'var(--text-primary)',
              cursor: 'pointer',
              fontSize: '13px',
              fontWeight: selectedDate === date ? 600 : 400,
            }}
          >
            {date}
          </button>
        ))}
      </div>

      {/* Articles for selected date */}
      {loading ? (
        <div className="loading">加载中...</div>
      ) : articles.length === 0 ? (
        <div className="empty">
          <p>该日期无文章</p>
        </div>
      ) : (
        <div className="article-list">
          {articles.map((article) => (
            <ArticleCard
              key={article.id}
              article={article}
              onStatusChange={handleStatusChange}
              currentTab={currentTab}
            />
          ))}
        </div>
      )}
    </div>
  );
}
