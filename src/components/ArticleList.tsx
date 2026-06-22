import { useEffect, useMemo, useRef, useState } from 'react';
import { RefreshCw, CheckCheck, Trash2, Bookmark, BookmarkCheck, Link2, ExternalLink, Circle, CheckCircle2 } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import type { Feed, Article } from '../types';
import { FeedAvatar } from './FeedAvatar';
import { useContextMenuPosition } from '../hooks/useContextMenuPosition';

interface ArticleListProps {
  articles: Article[];
  selectedArticle: Article | null;
  onArticleSelect: (article: Article) => void;
  loading: boolean;
  selectedFeed: Feed | null;
  /** All known feeds — used to show the source feed name in multi-feed views. */
  feeds?: Feed[];
  /** Show the feed's avatar in each article row. Useful in multi-feed views like Bookmarks. */
  showFeedAvatar?: boolean;
  title?: string;
  onRefreshFeed: () => void;
  onMarkAllRead?: () => void;
  onDeleteFeed?: () => void;
  onMarkArticleRead?: (articleId: string, isRead: boolean) => void;
  onToggleBookmark?: (articleId: string) => void;
  onCopyUrl?: (url: string) => void;
}

export function ArticleList({
  articles,
  selectedArticle,
  onArticleSelect,
  loading,
  selectedFeed,
  feeds,
  showFeedAvatar,
  title,
  onRefreshFeed,
  onMarkAllRead,
  onDeleteFeed,
  onMarkArticleRead,
  onToggleBookmark,
  onCopyUrl
}: ArticleListProps) {
  const listRef = useRef<HTMLDivElement>(null);
  const [contextMenu, setContextMenu] = useState<{ article: Article; x: number; y: number } | null>(null);

  // Build a fast id→feed lookup for multi-feed meta / avatars.
  const feedById = useMemo(() => {
    const map: Record<string, Feed> = {};
    for (const f of feeds ?? []) map[f.id] = f;
    return map;
  }, [feeds]);

  // Scroll the active article row into view whenever selection changes
  useEffect(() => {
    if (!selectedArticle || !listRef.current) return;
    const el = listRef.current.querySelector<HTMLElement>(`[data-article-id="${selectedArticle.id}"]`);
    el?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  }, [selectedArticle]);

  // Close article context menu on outside click or scroll
  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    document.addEventListener('mousedown', close);
    document.addEventListener('scroll', close, true);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('scroll', close, true);
    };
  }, [contextMenu]);

  const handleContextMenu = (e: React.MouseEvent, article: Article) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ article, x: e.clientX, y: e.clientY });
  };

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
        <div className="article-list-header-row">
          <h2 className="font-semibold truncate article-list-title">
            {title ?? (selectedFeed ? selectedFeed.title : 'All Articles')}
          </h2>
          <div className="article-list-actions">
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
            {onDeleteFeed && (
              <button
                className="btn btn-icon btn-ghost"
                onClick={onDeleteFeed}
                disabled={loading}
                title="Delete feed"
              >
                <Trash2 size={16} />
              </button>
            )}
          </div>
        </div>
      </div>

      <div className="list" ref={listRef}>
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
              data-article-id={article.id}
              className={`article-item ${selectedArticle?.id === article.id ? 'active' : ''} ${!article.is_read ? 'unread' : 'read'}`}
              onClick={() => onArticleSelect(article)}
              onContextMenu={e => handleContextMenu(e, article)}
            >
              <div className="article-header">
                {showFeedAvatar && feedById[article.feed_id] && (
                  <FeedAvatar feed={feedById[article.feed_id]} className="article-avatar" />
                )}
                <div className="article-content">
                  <div className="article-title">
                    {article.title}
                  </div>
                  
                  <div className="article-meta">
                    <span className="text-xs">
                      {formatDate(article.published_at)}
                    </span>
                    {feedById[article.feed_id]?.title && (
                      <span className="text-xs">
                        • {feedById[article.feed_id].title}
                      </span>
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
            </div>
          ))
        )}
      </div>

      {contextMenu && (
        <ArticleContextMenu
          article={contextMenu.article}
          x={contextMenu.x}
          y={contextMenu.y}
          onClose={() => setContextMenu(null)}
          onMarkArticleRead={onMarkArticleRead}
          onToggleBookmark={onToggleBookmark}
          onCopyUrl={onCopyUrl}
        />
      )}
    </div>
  );
}

interface ArticleContextMenuProps {
  article: Article;
  x: number;
  y: number;
  onClose: () => void;
  onMarkArticleRead?: (articleId: string, isRead: boolean) => void;
  onToggleBookmark?: (articleId: string) => void;
  onCopyUrl?: (url: string) => void;
}

const ArticleContextMenu = ({
  article,
  x,
  y,
  onClose,
  onMarkArticleRead,
  onToggleBookmark,
  onCopyUrl,
}: ArticleContextMenuProps) => {
  const { ref, style } = useContextMenuPosition(x, y);

  return (
    <div
      ref={ref}
      className="context-menu"
      style={{ position: 'fixed', ...style }}
      onMouseDown={e => e.stopPropagation()}
    >
      {onMarkArticleRead && (
        <button
          className="context-menu-item"
          onClick={() => { onMarkArticleRead(article.id, !article.is_read); onClose(); }}
        >
          {article.is_read ? <Circle size={12} /> : <CheckCircle2 size={12} />}
          {article.is_read ? 'Mark as unread' : 'Mark as read'}
        </button>
      )}

      {onToggleBookmark && (
        <button
          className="context-menu-item"
          onClick={() => { onToggleBookmark(article.id); onClose(); }}
        >
          {article.is_bookmarked ? <BookmarkCheck size={12} /> : <Bookmark size={12} />}
          {article.is_bookmarked ? 'Remove bookmark' : 'Bookmark article'}
        </button>
      )}

      {article.link && (
        <>
          <button
            className="context-menu-item"
            onClick={() => { onCopyUrl?.(article.link!); onClose(); }}
          >
            <Link2 size={12} />
            Copy link
          </button>

          <button
            className="context-menu-item"
            onClick={() => { openUrl(article.link!).catch(console.error); onClose(); }}
          >
            <ExternalLink size={12} />
            Open in browser
          </button>
        </>
      )}
    </div>
  );
};