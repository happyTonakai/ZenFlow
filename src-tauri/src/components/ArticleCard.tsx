import { useState } from 'react';
import { Article, ArticleStatus } from '../types/article';
import { updateArticleStatus, addComment } from '../hooks/useArticles';
import { openUrl } from '@tauri-apps/plugin-opener';
import katex from 'katex';
import 'katex/dist/katex.min.css';
import './ArticleCard.css';

interface ArticleCardProps {
  article: Article;
  onStatusChange?: (articleId: string, newStatus: number) => void;
  currentTab: number | null; // 当前所在标签页
}

// Split a string into alternating [text, math, text, math, ...] tokens.
// Supports: $$...$$, $...$, \[...\], \(...\)
function splitLatex(input: string): { math: boolean; display: boolean; content: string }[] {
  const RE = /\$\$([\s\S]+?)\$\$|\$([^$\n]+?)\$|\\\[([\s\S]+?)\\\]|\\\(([\s\S]+?)\\\)/g;
  const tokens: { math: boolean; display: boolean; content: string }[] = [];
  let lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = RE.exec(input)) !== null) {
    if (match.index > lastIndex) {
      tokens.push({ math: false, display: false, content: input.slice(lastIndex, match.index) });
    }
    const isDisplay = match[1] !== undefined || match[3] !== undefined;
    const mathContent = match[1] ?? match[2] ?? match[3] ?? match[4] ?? '';
    tokens.push({ math: true, display: isDisplay, content: mathContent });
    lastIndex = RE.lastIndex;
  }
  if (lastIndex < input.length) {
    tokens.push({ math: false, display: false, content: input.slice(lastIndex) });
  }
  return tokens;
}

interface LatexTextProps {
  text: string;
  className?: string;
  onClick?: () => void;
}

// Renders a string that may contain LaTeX math delimiters.
function LatexText({ text, className, onClick }: LatexTextProps) {
  const tokens = splitLatex(text);
  return (
    <span className={className} onClick={onClick}>
      {tokens.map((token, i) => {
        if (!token.math) {
          return <span key={i}>{token.content}</span>;
        }
        try {
          const html = katex.renderToString(token.content, {
            displayMode: token.display,
            throwOnError: false,
            output: 'html',
          });
          return (
            <span
              key={i}
              dangerouslySetInnerHTML={{ __html: html }}
              style={token.display ? { display: 'block', textAlign: 'center', margin: '4px 0' } : undefined}
            />
          );
        } catch {
          // Fallback: render raw source
          return <span key={i}>{token.display ? `$$${token.content}$$` : `$${token.content}$`}</span>;
        }
      })}
    </span>
  );
}

// 格式化作者：超过5个显示前3个+后2个+et. al.
function formatAuthors(author: string | null): string {
  if (!author) return '';
  const authors = author.split(',').map(a => a.trim());
  if (authors.length <= 5) {
    return author;
  }
  return `${authors.slice(0, 3).join(', ')}, ${authors.slice(-2).join(', ')}, et. al.`;
}

export function ArticleCard({ article, onStatusChange, currentTab: _currentTab }: ArticleCardProps) {
  const [showComment, setShowComment] = useState(false);
  const [commentText, setCommentText] = useState(article.comment || '');
  const [savingComment, setSavingComment] = useState(false);

  const handleSaveComment = async () => {
    setSavingComment(true);
    try {
      await addComment(article.id, commentText);
      setShowComment(false);
    } catch (e) {
      console.error('Failed to save comment:', e);
    } finally {
      setSavingComment(false);
    }
  };

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
        {article.category && <span className="article-category">{article.category}</span>}
        {getStatusBadge()}
      </div>
      
      <h3 className="article-title">
        <LatexText text={article.translated_title || article.title} onClick={openLink} />
      </h3>
      
      {article.translated_title && article.title && (
        <p className="article-original-title">
          <LatexText text={article.title} />
        </p>
      )}
      
      {article.author && (
        <p className="article-author">
          {formatAuthors(article.author)}
        </p>
      )}
      
      {article.abstract && (
        <p className="article-abstract">
          <LatexText text={article.abstract} />
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
          <button
            className={`btn-comment ${article.comment ? 'has-comment' : ''}`}
            onClick={() => setShowComment(!showComment)}
            title={article.comment ? '编辑评论' : '添加评论'}
          >
            💬
          </button>
        </div>
      </div>

      {showComment && (
        <div className="comment-section">
          <textarea
            className="comment-input"
            value={commentText}
            onChange={(e) => setCommentText(e.target.value)}
            placeholder="写下你对这篇文章的看法，帮助 AI 更好地理解你的偏好..."
            rows={2}
          />
          <div className="comment-actions">
            <button
              className="btn-comment-save"
              onClick={handleSaveComment}
              disabled={savingComment}
            >
              {savingComment ? '保存中...' : '保存'}
            </button>
            <button
              className="btn-comment-cancel"
              onClick={() => {
                setShowComment(false);
                setCommentText(article.comment || '');
              }}
            >
              取消
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
