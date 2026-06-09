import { Plus, RefreshCw, Rss } from 'lucide-react';
import type { Feed } from '../types';

interface SidebarProps {
  feeds: Feed[];
  selectedFeed: Feed | null;
  unreadCounts: Record<string, number>;
  onFeedSelect: (feed: Feed) => void;
  onAddFeed: () => void;
  onRefreshAll: () => void;
  loading: boolean;
}

export function Sidebar({
  feeds,
  selectedFeed,
  unreadCounts,
  onFeedSelect,
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
            <span className="feed-unread-count">
              {totalUnread}
            </span>
          )}
        </div>
      </div>

      <div className="list">
        {feeds.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📡</div>
            <div className="empty-state-title">No feeds yet</div>
            <div className="empty-state-description">
              Add your first RSS or Atom feed to get started
            </div>
            <button
              className="btn btn-primary mt-md"
              onClick={onAddFeed}
            >
              Add Feed
            </button>
          </div>
        ) : (
          feeds.map(feed => (
            <div
              key={feed.id}
              className={`list-item ${selectedFeed?.id === feed.id ? 'active' : ''}`}
              onClick={() => onFeedSelect(feed)}
            >
              <div className="feed-item">
                <div className="feed-title truncate">
                  {feed.title}
                </div>
                {unreadCounts[feed.id] > 0 && (
                  <span className="feed-unread-count">
                    {unreadCounts[feed.id]}
                  </span>
                )}
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