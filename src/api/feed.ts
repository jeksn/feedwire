import { invoke } from '@tauri-apps/api/core';
import type { Feed, Article } from '../types';

export interface FailedFeed {
  url: string;
  title: string;
  reason: string;
}

export interface ImportResult {
  added: number;
  skipped: number;
  failed: FailedFeed[];
}

export const feedApi = {
  // Feed operations
  addFeed: async (url: string): Promise<Feed> => {
    return await invoke('add_feed', { url });
  },

  getFeeds: async (): Promise<Feed[]> => {
    return await invoke('get_feeds');
  },

  deleteFeed: async (feedId: string): Promise<void> => {
    return await invoke('delete_feed', { feedId });
  },

  deleteAllFeeds: async (): Promise<number> => {
    return await invoke('delete_all_feeds');
  },

  refreshFeed: async (feedId: string): Promise<Article[]> => {
    return await invoke('refresh_feed', { feedId });
  },

  refreshAllFeeds: async (): Promise<string[]> => {
    return await invoke('refresh_all_feeds');
  },

  getUnreadCount: async (feedId?: string): Promise<number> => {
    return await invoke('get_unread_count', { feedId });
  },

  // Article operations
  getArticles: async (
    feedId?: string,
    limit?: number,
    offset?: number
  ): Promise<Article[]> => {
    return await invoke('get_articles', { feedId, limit, offset });
  },

  getArticle: async (articleId: string): Promise<Article> => {
    return await invoke('get_article', { articleId });
  },

  markArticleRead: async (articleId: string, isRead: boolean): Promise<Article> => {
    return await invoke('mark_article_read', { articleId, isRead });
  },

  toggleBookmark: async (articleId: string): Promise<Article> => {
    return await invoke('toggle_bookmark', { articleId });
  },

  getBookmarkedArticles: async (): Promise<Article[]> => {
    return await invoke('get_bookmarked_articles');
  },

  getUnreadArticles: async (): Promise<Article[]> => {
    return await invoke('get_unread_articles');
  },

  markAllRead: async (feedId?: string): Promise<void> => {
    return await invoke('mark_all_read', { feedId });
  },

  exportOpml: async (): Promise<string> => {
    return await invoke('export_opml');
  },

  importOpml: async (): Promise<ImportResult> => {
    return await invoke('import_opml');
  },
};