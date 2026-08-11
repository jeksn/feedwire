import { useState, useEffect } from 'react';
import { X, RotateCcw, Archive } from 'lucide-react';
import type { Feed } from '../types';
import { feedApi } from '../api/feed';
import { FeedAvatar } from './FeedAvatar';

interface ArchiveDialogProps {
  onClose: () => void;
  onRestored: () => void;
}

export function ArchiveDialog({ onClose, onRestored }: ArchiveDialogProps) {
  const [archivedFeeds, setArchivedFeeds] = useState<Feed[]>([]);
  const [loading, setLoading] = useState(true);
  const [restoringId, setRestoringId] = useState<string | null>(null);

  const loadArchived = async () => {
    try {
      setLoading(true);
      const feeds = await feedApi.getArchivedFeeds();
      setArchivedFeeds(feeds);
    } catch (error) {
      console.error('Failed to load archived feeds:', error);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadArchived();
  }, []);

  const handleRestore = async (feedId: string) => {
    try {
      setRestoringId(feedId);
      await feedApi.unarchiveFeed(feedId);
      setArchivedFeeds(prev => prev.filter(f => f.id !== feedId));
      onRestored();
    } catch (error) {
      console.error('Failed to restore feed:', error);
    } finally {
      setRestoringId(null);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose();
    }
  };

  return (
    <div className="dialog-overlay" onKeyDown={handleKeyDown} tabIndex={0}>
      <div className="dialog">
        <div className="dialog-header">
          <h2 className="text-lg font-semibold">Archived Feeds</h2>
          <button
            className="btn btn-icon btn-ghost"
            onClick={onClose}
            title="Close"
          >
            <X size={18} />
          </button>
        </div>

        <div className="dialog-body">
          {loading ? (
            <div className="text-sm text-secondary">Loading archived feeds…</div>
          ) : archivedFeeds.length === 0 ? (
            <div className="empty-state">
              <div className="empty-state-icon"><Archive size={32} /></div>
              <div className="empty-state-title">No archived feeds</div>
              <div className="empty-state-description">
                Archived feeds will appear here. You can restore them anytime.
              </div>
            </div>
          ) : (
            <div className="archive-feed-list">
              {archivedFeeds.map(feed => (
                <div key={feed.id} className="archive-feed-item">
                  <FeedAvatar feed={feed} className="archive-feed-avatar" />
                  <div className="archive-feed-info">
                    <div className="archive-feed-title truncate">{feed.title}</div>
                    <div className="archive-feed-url truncate">{feed.url}</div>
                  </div>
                  <button
                    className="btn btn-icon btn-ghost"
                    onClick={() => handleRestore(feed.id)}
                    disabled={restoringId === feed.id}
                    title="Restore feed"
                  >
                    <RotateCcw size={16} className={restoringId === feed.id ? 'animate-spin' : ''} />
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="dialog-footer">
          <button
            type="button"
            className="btn btn-secondary"
            onClick={onClose}
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
