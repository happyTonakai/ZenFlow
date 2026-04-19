// 文章状态枚举
export const ArticleStatus = {
  UNREAD: 0,
  CLICKED: 1,
  LIKED: 2,
  MARKED_READ: 3,
  DISLIKED: -1,
} as const;

export type ArticleStatusType = typeof ArticleStatus[keyof typeof ArticleStatus];

// 文章数据类型
export interface Article {
  id: string;
  title: string;
  link: string;
  abstract: string | null;
  source: string;
  status: number;
  score: number;
  translated_title: string | null;
  translated_abstract: string | null;
  hf_upvotes: number | null;
  ax_upvotes: number | null;
  ax_downvotes: number | null;
  timestamp: string | null;
  author: string | null;
  category: string | null;
  comment: string | null;
  recommendationType?: 'score' | 'diversity';
  recommendDate: string | null;
  batchOrder: number | null;
}

// 统计数据
export interface Stats {
  unread: number;
  clicked: number;
  liked: number;
  marked_read: number;
  disliked: number;
  initialized: boolean;
}

// 刷新结果
export interface RefreshResult {
  preferences_updated: boolean;
  scores_updated: number;
}
