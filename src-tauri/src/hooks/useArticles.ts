import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Article, ArticleStatus, Stats } from '../types/article';

export function useArticles(status: number | null, limit: number = 50) {
  const [articles, setArticles] = useState<Article[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchArticles = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<Article[]>('get_articles', {
        status,
        limit,
        offset: 0,
      });
      setArticles(result);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [status, limit]);

  useEffect(() => {
    fetchArticles();
  }, [fetchArticles]);

  // 就地更新文章状态
  // 根据当前标签页决定是否移除文章：
  // - 未读页面：点击不消失，点赞/点踩/跳过才消失
  // - 喜欢页面：取消喜欢或改成其他状态才消失
  // - 全部页面：永远不消失，只更新状态
  const updateArticleStatusLocal = useCallback((articleId: string, newStatus: number) => {
    setArticles(prev => {
      const article = prev.find(a => a.id === articleId);
      if (!article) return prev;
      
      // 如果状态没有变化，不处理
      if (article.status === newStatus) return prev;
      
      // 全部页面：永远不消失，只更新状态
      if (status === null) {
        return prev.map(a => 
          a.id === articleId 
            ? { ...a, status: newStatus }
            : a
        );
      }
      
      // 未读页面：点击不消失，其他状态变化才消失
      if (status === ArticleStatus.UNREAD) {
        if (newStatus === ArticleStatus.CLICKED) {
          // 点击：不消失，更新状态
          return prev.map(a => 
            a.id === articleId 
              ? { ...a, status: newStatus }
              : a
          );
        }
        // 点赞/点踩/跳过：消失
        return prev.filter(a => a.id !== articleId);
      }
      
      // 喜欢页面：只有离开"喜欢"状态才消失
      if (status === ArticleStatus.LIKED) {
        if (newStatus === ArticleStatus.LIKED) {
          return prev; // 保持喜欢，不处理
        }
        // 变成其他状态：消失
        return prev.filter(a => a.id !== articleId);
      }
      
      // 其他页面：状态变化时消失
      return prev.filter(a => a.id !== articleId);
    });
  }, [status]);

  return { 
    articles, 
    loading, 
    error, 
    refetch: fetchArticles,
    updateArticleStatusLocal 
  };
}



export function useStats() {
  const [stats, setStats] = useState<Stats | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchStats = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<Stats>('get_stats');
      setStats(result);
    } catch (e) {
      console.error('Failed to fetch stats:', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStats();
  }, [fetchStats]);

  return { stats, loading, refetch: fetchStats };
}

export async function fetchNewArticles(): Promise<number> {
  return await invoke<number>('fetch_articles');
}

export async function updateArticleStatus(id: string, status: number): Promise<void> {
  await invoke('update_status', { articleId: id, status });
}

export async function markAllRead(): Promise<number> {
  return await invoke<number>('mark_all_read');
}

export async function refreshRecommendations() {
  return await invoke<{
    pos_count: number;
    neg_count: number;
    pos_clusters: number;
    neg_clusters: number;
    scores_updated: number;
  }>('refresh_recommendations');
}

export async function checkInitialized(): Promise<boolean> {
  return await invoke<boolean>('is_initialized');
}

// ========== 初始化和设置 ==========

export interface AppSettings {
  arxiv_categories: string[];
  siliconflow_api_key: string;
  pos_clusters: number;
  neg_clusters: number;
  daily_papers: number;
  negative_alpha: number;
  diversity_ratio: number;
  enable_translation: boolean;
  translation_model: string;
}

export interface InitRequest {
  arxiv_categories: string[];
  favorite_papers: string[];
  siliconflow_api_key: string;
  pos_clusters: number;
  neg_clusters: number;
  daily_papers: number;
  negative_alpha: number;
  diversity_ratio: number;
  enable_translation: boolean;
}

export interface InitResult {
  settings_saved: boolean;
  papers_fetched: number;
  papers_embedded: number;
  pos_clusters: number;
  neg_clusters: number;
  errors: string[];
}

export async function needsInitialization(): Promise<boolean> {
  return await invoke<boolean>('needs_initialization');
}

export async function getSettings(): Promise<AppSettings> {
  return await invoke<AppSettings>('get_settings');
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  await invoke('save_settings', { settings });
}

export async function initializeApp(request: InitRequest): Promise<InitResult> {
  return await invoke<InitResult>('initialize_app', { request });
}

export async function getArxivCategories(): Promise<string[]> {
  return await invoke<string[]>('get_arxiv_categories');
}

export async function translateText(text: string, apiKey?: string): Promise<string> {
  return await invoke<string>('translate_text', { text, apiKey });
}