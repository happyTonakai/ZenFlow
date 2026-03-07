import { Article, ArticleStatus } from '../types/article';
import { updateArticleStatus } from '../hooks/useArticles';
import { openUrl } from '@tauri-apps/plugin-opener';
import './ArticleCard.css';

interface ArticleCardProps {
  article: Article;
  onStatusChange?: (articleId: string, newStatus: number) => void;
  currentTab: number | null; // 当前所在标签页
}

export function ArticleCard({ article, onStatusChange, currentTab }: ArticleCardProps) {
  // 处理点赞/点踩/跳过
  const handleStatus = async (newStatus: number) => {
    try {
      // 如果点击的是当前状态，则取消（回到未读）
      const finalStatus = article.status === newStatus ? ArticleStatus.UNREAD : newStatus;
      await updateArticleStatus(article.id, finalStatus);
      // 通知父组件状态变化
      onStatusChange?.(article.id, finalStatus);
    } catch (e) {
      console.error('Failed to update status:', e);
    }
  };

  // 点击标题打开链接
  const openLink = async () => {
    // 在默认浏览器中打开
    try {
      await openUrl(article.link);
    } catch (e) {
      console.error('Failed to open URL:', e);
      window.open(article.link, '_blank');
    }
    
    // 点开操作：只有未读状态才变成点击
    // 点赞/点踩状态下点开不覆盖
    if (article.status === ArticleStatus.UNREAD) {
      try {
        await updateArticleStatus(article.id, ArticleStatus.CLICKED);
        onStatusChange?.(article.id, ArticleStatus.CLICKED);
      } catch (e) {
        console.error('Failed to update status:', e);
      }
    }
  };

  // 根据状态获取样式类
  const getStatusClass = () => {
    switch (article.status) {
      case ArticleStatus.LIKED:
        return 'status-liked';
      case ArticleStatus.DISLIKED:
        return 'status-disliked';
      case ArticleStatus.CLICKED:
        return 'status-clicked';
      case ArticleStatus.MARKED_READ:
        return 'status-skipped';
      default:
        return '';
    }
  };

  // 获取状态标签
  const getStatusBadge = () => {
    switch (article.status) {
      case ArticleStatus.LIKED:
        return <span className="status-badge liked">👍 已喜欢</span>;
      case ArticleStatus.DISLIKED:
        return <span className="status-badge disliked">👎 不喜欢</span>;
      case ArticleStatus.CLICKED:
        return <span className="status-badge clicked">📖 已点击</span>;
      case ArticleStatus.MARKED_READ:
        return <span className="status-badge skipped">✓ 已跳过</span>;
      default:
        return null;
    }
  };

  return (
    <div className={`article-card ${getStatusClass()}`}>
      <div className="article-header">
        <span className="article-source">{article.source}</span>
        <span className="article-score">Score: {article.score.toFixed(3)}</span>
        {getStatusBadge()}
      </div>
      
      <h3 className="article-title" onClick={openLink}>
        {article.title}
      </h3>
      
      {article.abstract && (
        <p className="article-abstract">
          {article.abstract.slice(0, 300)}
          {article.abstract.length > 300 ? '...' : ''}
        </p>
      )}
      
      <div className="article-footer">
        <div className="article-stats">
          {article.hf_upvotes !== null && (
            <span className="stat">HF: {article.hf_upvotes}👍</span>
          )}
          {article.ax_upvotes !== null && (
            <span className="stat">AX: {article.ax_upvotes}/{article.ax_downvotes}</span>
          )}
        </div>
        
        {/* 始终显示操作按钮，根据状态高亮 */}
        <div className="article-actions">
          <button 
            className={`btn-like ${article.status === ArticleStatus.LIKED ? 'active' : ''}`}
            onClick={() => handleStatus(ArticleStatus.LIKED)}
            title="喜欢"
          >
            👍
          </button>
          <button 
            className={`btn-dislike ${article.status === ArticleStatus.DISLIKED ? 'active' : ''}`}
            onClick={() => handleStatus(ArticleStatus.DISLIKED)}
            title="不喜欢"
          >
            👎
          </button>
          <button 
            className={`btn-skip ${article.status === ArticleStatus.MARKED_READ ? 'active' : ''}`}
            onClick={() => handleStatus(ArticleStatus.MARKED_READ)}
            title="跳过（不参与推荐）"
          >
            →
          </button>
        </div>
      </div>
    </div>
  );
}
