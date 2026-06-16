import { Plus, RefreshCw, Rss, Bookmark, Inbox, Settings } from 'lucide-react';
import type { Feed } from '../types';

interface SidebarProps {
  feeds: Feed[];
  selectedFeed: Feed | null;
  selectedView: 'feed' | 'unread' | 'bookmarks' | 'settings';
  unreadCounts: Record<string, number>;
  onFeedSelect: (feed: Feed) => void;
  onUnreadSelect: () => void;
  onBookmarksSelect: () => void;
  onSettingsSelect: () => void;
  onAddFeed: () => void;
  onRefreshAll: () => void;
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
  onSettingsSelect,
  onAddFeed,
  onRefreshAll,
  loading
}: SidebarProps) {
  const totalUnread = Object.values(unreadCounts).reduce((sum, count) => sum + count, 0);

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

      <div className="sidebar-nav">
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
      </div>

      <div className="sidebar-section-divider" />

      <div className="list">
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
            >
              <div className="feed-item">
                <div className="feed-title truncate">{feed.title}</div>
                {unreadCounts[feed.id] > 0 && (
                  <span className="feed-unread-count">{unreadCounts[feed.id]}</span>
                )}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Settings pinned at bottom */}
      <div className="sidebar-footer">
        <div className="sidebar-section-divider" />
        <div
          className={`list-item ${selectedView === 'settings' ? 'active' : ''}`}
          onClick={onSettingsSelect}
        >
          <div className="feed-item">
            <div className="flex items-center gap-sm feed-title">
              <Settings size={14} />
              <span>Settings</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
