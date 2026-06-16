import { RefreshCw, ExternalLink, CheckCheck } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import type { Feed, Article } from '../types';

interface ArticleListProps {
  articles: Article[];
  selectedArticle: Article | null;
  onArticleSelect: (article: Article) => void;
  loading: boolean;
  selectedFeed: Feed | null;
  title?: string;
  onRefreshFeed: () => void;
  onMarkAllRead?: () => void;
}

export function ArticleList({
  articles,
  selectedArticle,
  onArticleSelect,
  loading,
  selectedFeed,
  title,
  onRefreshFeed,
  onMarkAllRead
}: ArticleListProps) {
  const formatDate = (dateString?: string) => {
    if (!dateString) return '';
    
    const date = new Date(dateString);
    const now = new Date();
    const diffInHours = (now.getTime() - date.getTime()) / (1000 * 60 * 60);
    
    if (diffInHours < 24) {
      return date.toLocaleTimeString('en-US', { 
        hour: 'numeric', 
        minute: '2-digit',
        hour12: true 
      });
    } else if (diffInHours < 24 * 7) {
      return date.toLocaleDateString('en-US', { 
        weekday: 'short' 
      });
    } else {
      return date.toLocaleDateString('en-US', { 
        month: 'short', 
        day: 'numeric' 
      });
    }
  };

  const truncateText = (text: string, maxLength: number) => {
    // Strip HTML tags for snippet display
    const plain = text.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim();
    if (plain.length <= maxLength) return plain;
    return plain.substring(0, maxLength).trim() + '...';
  };

  return (
    <div className="article-list">
      <div className="header">
        <div className="flex items-center justify-between gap-x-4">
          <h2 className="font-semibold">
            {title ?? (selectedFeed ? selectedFeed.title : 'All Articles')}
          </h2>
          <div className="flex gap-2">
            {onMarkAllRead && articles.some(a => !a.is_read) && (
              <button
                className="btn btn-icon btn-ghost"
                onClick={onMarkAllRead}
                disabled={loading}
                title="Mark all as read"
              >
                <CheckCheck size={16} />
              </button>
            )}
            <button
              className="btn btn-icon btn-ghost"
              onClick={onRefreshFeed}
              disabled={loading}
              title="Refresh feed"
            >
              <RefreshCw size={16} className={loading ? 'animate-spin' : ''} />
            </button>
          </div>
        </div>
      </div>

      <div className="list">
        {loading ? (
          <div className="loading">
            <div className="spinner"></div>
            <span className="ml-sm">Loading articles...</span>
          </div>
        ) : articles.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📄</div>
            <div className="empty-state-title">No articles</div>
            <div className="empty-state-description">
              {selectedFeed 
                ? 'No articles found in this feed'
                : 'Select a feed to see articles'
              }
            </div>
          </div>
        ) : (
          articles.map(article => (
            <div
              key={article.id}
              className={`article-item ${selectedArticle?.id === article.id ? 'active' : ''} ${!article.is_read ? 'unread' : 'read'}`}
              onClick={() => onArticleSelect(article)}
            >
              <div className="article-header">
                <div className="article-title">
                  {article.title}
                </div>
                
                <div className="article-meta">
                  <span className="text-xs">
                    {formatDate(article.published_at)}
                  </span>
                  {article.author && (
                    <span className="text-xs">
                      • {article.author}
                    </span>
                  )}
                  {article.link && (
                    <button
                      className="btn btn-icon btn-ghost ml-auto"
                      onClick={(e) => {
                        e.stopPropagation();
                        openUrl(article.link!).catch(console.error);
                      }}
                      title="Open in browser"
                    >
                      <ExternalLink size={12} />
                    </button>
                  )}
                </div>
                
                {(article.description || article.content) && (
                  <div className="article-snippet">
                    {truncateText(
                      article.description || article.content || '',
                      120
                    )}
                  </div>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}