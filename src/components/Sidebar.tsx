import { useState } from 'react';
import { Plus, RefreshCw, Rss, Trash2, Bookmark, Inbox } from 'lucide-react';
import type { Feed } from '../types';

interface SidebarProps {
  feeds: Feed[];
  selectedFeed: Feed | null;
  selectedView: 'feed' | 'unread' | 'bookmarks';
  unreadCounts: Record<string, number>;
  onFeedSelect: (feed: Feed) => void;
  onUnreadSelect: () => void;
  onBookmarksSelect: () => void;
  onAddFeed: () => void;
  onRefreshAll: () => void;
  onDeleteFeed: (feedId: string) => void;
  loading: boolean;
}

export function Sidebar({
  feeds,
  selectedFeed,
  selectedView,
  unreadCounts,
  onFeedSelect,
  onUnreadSelect,
  onBookmarksSelect,
  onAddFeed,
  onRefreshAll,
  onDeleteFeed,
  loading
}: SidebarProps) {
  const [hoveredFeedId, setHoveredFeedId] = useState<string | null>(null);
  const [confirmDeleteId, setConfirmDeleteId] = useState<string | null>(null);

  const totalUnread = Object.values(unreadCounts).reduce((sum, count) => sum + count, 0);

  const handleDeleteClick = (e: React.MouseEvent, feedId: string) => {
    e.stopPropagation();
    if (confirmDeleteId === feedId) {
      onDeleteFeed(feedId);
      setConfirmDeleteId(null);
    } else {
      setConfirmDeleteId(feedId);
      // Auto-cancel confirmation after 3s
      setTimeout(() => setConfirmDeleteId(null), 3000);
    }
  };

  return (
    <div className="sidebar">
      <div className="sidebar-header">
        <div className="flex items-center justify-between">
          <h1 className="text-lg font-semibold">FeedWire</h1>
          <div className="flex gap-xs">
            <button
              className="btn btn-icon btn-ghost"
              onClick={onRefreshAll}
              disabled={loading}
              title="Refresh all feeds"
            >
              <RefreshCw size={16} className={loading ? 'animate-spin' : ''} />
            </button>
            <button
              className="btn btn-icon btn-primary"
              onClick={onAddFeed}
              title="Add feed"
            >
              <Plus size={16} />
            </button>
          </div>
        </div>

        <div className="flex items-center gap-sm mt-md">
          <Rss size={16} className="text-secondary" />
          <span className="text-sm text-secondary">
            {feeds.length} {feeds.length === 1 ? 'feed' : 'feeds'}
          </span>
          {totalUnread > 0 && (
            <span className="feed-unread-count">{totalUnread}</span>
          )}
        </div>
      </div>

      <div className="list">
        {/* Unread section */}
        <div
          className={`list-item ${selectedView === 'unread' ? 'active' : ''}`}
          onClick={onUnreadSelect}
        >
          <div className="feed-item">
            <div className="flex items-center gap-sm feed-title">
              <Inbox size={14} />
              <span>Unread</span>
            </div>
            {totalUnread > 0 && (
              <span className="feed-unread-count">{totalUnread}</span>
            )}
          </div>
        </div>

        {/* Bookmarks section */}
        <div
          className={`list-item ${selectedView === 'bookmarks' ? 'active' : ''}`}
          onClick={onBookmarksSelect}
        >
          <div className="feed-item">
            <div className="flex items-center gap-sm feed-title">
              <Bookmark size={14} />
              <span>Bookmarks</span>
            </div>
          </div>
        </div>

        <div className="sidebar-section-divider" />

        {feeds.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📡</div>
            <div className="empty-state-title">No feeds yet</div>
            <div className="empty-state-description">
              Add your first RSS or Atom feed to get started
            </div>
            <button className="btn btn-primary mt-md" onClick={onAddFeed}>
              Add Feed
            </button>
          </div>
        ) : (
          feeds.map(feed => (
            <div
              key={feed.id}
              className={`list-item ${selectedView === 'feed' && selectedFeed?.id === feed.id ? 'active' : ''}`}
              onClick={() => onFeedSelect(feed)}
              onMouseEnter={() => setHoveredFeedId(feed.id)}
              onMouseLeave={() => { setHoveredFeedId(null); setConfirmDeleteId(null); }}
            >
              <div className="feed-item">
                <div className="feed-title truncate">{feed.title}</div>
                <div className="flex items-center gap-xs">
                  {unreadCounts[feed.id] > 0 && (
                    <span className="feed-unread-count">{unreadCounts[feed.id]}</span>
                  )}
                  {(hoveredFeedId === feed.id || confirmDeleteId === feed.id) && (
                    <button
                      className={`btn btn-icon btn-ghost feed-delete-btn ${confirmDeleteId === feed.id ? 'feed-delete-confirm' : ''}`}
                      onClick={(e) => handleDeleteClick(e, feed.id)}
                      title={confirmDeleteId === feed.id ? 'Click again to confirm delete' : 'Delete feed'}
                    >
                      <Trash2 size={13} />
                    </button>
                  )}
                </div>
              </div>
              {feed.description && (
                <div className="text-xs text-secondary truncate mt-xs">
                  {feed.description}
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
