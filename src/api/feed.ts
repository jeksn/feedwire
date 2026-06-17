import { invoke } from '@tauri-apps/api/core';
import type { Feed, Article, Folder, FilterRule, FilterSettings, FilterField } from '../types';

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

  // Folder operations
  getFolders: async (): Promise<Folder[]> => {
    return await invoke('get_folders');
  },

  createFolder: async (name: string): Promise<Folder> => {
    return await invoke('create_folder', { name });
  },

  renameFolder: async (folderId: string, name: string): Promise<Folder> => {
    return await invoke('rename_folder', { folderId, name });
  },

  deleteFolder: async (folderId: string): Promise<void> => {
    return await invoke('delete_folder', { folderId });
  },

  setFeedFolder: async (feedId: string, folderId: string | null): Promise<Feed> => {
    return await invoke('set_feed_folder', { feedId, folderId });
  },

  // Filter settings
  getFilterSettings: async (): Promise<FilterSettings> => {
    return await invoke('get_filter_settings');
  },

  setSkipYoutubeShorts: async (enabled: boolean): Promise<void> => {
    return await invoke('set_skip_youtube_shorts', { enabled });
  },

  addFilterRule: async (pattern: string, field: FilterField): Promise<FilterRule> => {
    return await invoke('add_filter_rule', { pattern, field });
  },

  updateFilterRuleEnabled: async (ruleId: string, enabled: boolean): Promise<void> => {
    return await invoke('update_filter_rule_enabled', { ruleId, enabled });
  },

  deleteFilterRule: async (ruleId: string): Promise<void> => {
    return await invoke('delete_filter_rule', { ruleId });
  },
};