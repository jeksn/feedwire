export interface Feed {
  id: string;
  title: string;
  url: string;
  description?: string;
  feed_type: string;
  last_fetched?: string;
  created_at: string;
  updated_at: string;
  is_active: boolean;
  /** ISO timestamp of the most recently published article for this feed. */
  latest_article_at?: string;
}

export interface Article {
  id: string;
  feed_id: string;
  title: string;
  link?: string;
  description?: string;
  content?: string;
  author?: string;
  published_at?: string;
  created_at: string;
  updated_at: string;
  is_read: boolean;
  is_bookmarked: boolean;
  guid?: string;
}

export interface NewFeed {
  title: string;
  url: string;
  description?: string;
  feed_type: string;
}

export interface FeedUpdate {
  title?: string;
  description?: string;
  last_fetched?: string;
  is_active?: boolean;
}

export interface ArticleUpdate {
  title?: string;
  link?: string;
  description?: string;
  content?: string;
  author?: string;
  published_at?: string;
  is_read?: boolean;
  is_bookmarked?: boolean;
}

export type FilterField = 'url' | 'title';

export interface FilterRule {
  id: string;
  pattern: string;
  field: FilterField;
  enabled: boolean;
  created_at: string;
}

export interface FilterSettings {
  skip_youtube_shorts: boolean;
  rules: FilterRule[];
}