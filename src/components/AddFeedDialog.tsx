import { useState } from 'react';
import { X, Link, Video } from 'lucide-react';

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
              <div className="relative">
                <input
                  type="url"
                  value={url}
                  onChange={(e) => setUrl(e.target.value)}
                  placeholder="https://example.com/feed.xml"
                  className="input pr-lg"
                  autoFocus
                  disabled={loading}
                />
                <div className="absolute inset-y-0 right-0 flex items-center pr-sm">
                  {url.includes('youtube.com') ? (
                    <Video size={16} className="text-secondary" />
                  ) : (
                    <Link size={16} className="text-secondary" />
                  )}
                </div>
              </div>
              {error && (
                <div className="text-xs text-error mt-xs">
                  {error}
                </div>
              )}
            </div>

            <div className="text-xs text-secondary">
              <p className="mb-sm">
                <strong>Supported formats:</strong> RSS, Atom, and YouTube channels
              </p>
              <p className="mb-sm">
                <strong>YouTube:</strong> Only direct channel URLs are supported (format: https://www.youtube.com/channel/CHANNEL_ID)
              </p>
              <p>
                <strong>Examples:</strong>
              </p>
              <ul className="ml-sm mt-xs">
                <li>• https://example.com/feed.xml</li>
                <li>• https://youtube.com/channel/UCBJycsmduvYEL83R_U4JriQ</li>
              </ul>
              <p className="mt-sm text-xs">
                <strong>Note:</strong> Custom YouTube URLs (@username) are not yet supported.
              </p>
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