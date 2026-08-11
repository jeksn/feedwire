import { useState } from 'react';
import { X } from 'lucide-react';

interface AddFeedDialogProps {
  onClose: () => void;
  onAddFeed: (url: string) => Promise<void>;
}

export function AddFeedDialog({ onClose, onAddFeed }: AddFeedDialogProps) {
  const [url, setUrl] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (!url.trim()) {
      setError('Please enter a feed URL');
      return;
    }

    try {
      setLoading(true);
      setError('');
      await onAddFeed(url.trim());
      onClose();
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Failed to add feed');
    } finally {
      setLoading(false);
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
          <h2 className="text-lg font-semibold">Add Feed</h2>
          <button
            className="btn btn-icon btn-ghost"
            onClick={onClose}
            title="Close"
          >
            <X size={18} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="dialog-body">
            <div className="mb-md">
              <label className="block text-sm font-medium mb-xs">
                Feed URL
              </label>
              <input
                type="url"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="https://example.com/feed.xml"
                className="input"
                autoFocus
                disabled={loading}
              />
              {error && (
                <div className="text-xs text-error mt-xs">
                  {error}
                </div>
              )}
            </div>

            <div className="add-feed-supported">
              <span className="add-feed-supported-label">Supported</span>
              <span className="add-feed-supported-tag">RSS</span>
              <span className="add-feed-supported-tag">Atom</span>
              <span className="add-feed-supported-tag">YouTube</span>
            </div>
          </div>

          <div className="dialog-footer">
            <button
              type="button"
              className="btn btn-secondary"
              onClick={onClose}
              disabled={loading}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="btn btn-primary"
              disabled={loading || !url.trim()}
            >
              {loading ? 'Adding...' : 'Add Feed'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}