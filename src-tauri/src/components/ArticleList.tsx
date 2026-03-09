import { Article } from '../types/article';
import { ArticleCard } from './ArticleCard';

interface ArticleListProps {
  articles: Article[];
  loading: boolean;
  error: string | null;
  onStatusChange: (articleId: string, newStatus: number) => void;
  currentTab: number | null;
}

export function ArticleList({ articles, loading, error, onStatusChange, currentTab }: ArticleListProps) {
  if (loading) {
    return <div className="loading">加载中...</div>;
  }

  if (error) {
    return (
      <div className="error">
        <p>加载失败: {error}</p>
      </div>
    );
  }

  if (articles.length === 0) {
    return (
      <div className="empty">
        <p>暂无文章</p>
        <p style={{ fontSize: '14px', marginTop: '8px' }}>
          点击上方「抓取新文章」按钮获取最新内容
        </p>
      </div>
    );
  }

  // 找到随机多样性部分开始的位置
  const diversityIndex = articles.findIndex(a => a.recommendationType === 'diversity');

  return (
    <div className="article-list">
      {articles.map((article, index) => (
        <>
          {index === diversityIndex && diversityIndex > 0 && (
            <div className="recommendation-divider" key="divider">
              <span>—— 为你推荐的随机多样性文章 ——</span>
            </div>
          )}
          <ArticleCard 
            key={article.id} 
            article={article}
            onStatusChange={onStatusChange}
            currentTab={currentTab}
          />
        </>
      ))}
    </div>
  );
}
