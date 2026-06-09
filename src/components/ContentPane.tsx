import { Bookmark, BookmarkCheck, ExternalLink, Calendar, User } from 'lucide-react';
import { openUrl as tauriOpenUrl } from '@tauri-apps/plugin-opener';
import type { Article } from '../types';

interface ContentPaneProps {
  article: Article | null;
  onToggleBookmark: (articleId: string) => void;
}

function openUrl(url: string) {
  tauriOpenUrl(url).catch(console.error);
}

export function ContentPane({ article, onToggleBookmark }: ContentPaneProps) {
  const formatDate = (dateString?: string) => {
    if (!dateString) return '';
    const date = new Date(dateString);
    if (isNaN(date.getTime())) return '';
    return date.toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit',
      hour12: true
    });
  };

  // Intercept all link clicks inside the article body and open them externally
  const handleContentClick = (e: React.MouseEvent<HTMLDivElement>) => {
    const target = (e.target as HTMLElement).closest('a');
    if (target?.href) {
      e.preventDefault();
      openUrl(target.href);
    }
  };

  if (!article) {
    return (
      <div className="content-pane">
        <div className="empty-state">
          <div className="empty-state-icon">📖</div>
          <div className="empty-state-title">No article selected</div>
          <div className="empty-state-description">
            Select an article from the list to read its content
          </div>
        </div>
      </div>
    );
  }

  const bodyHtml = article.content || article.description || '';
  const hasHtml = /<[a-z][\s\S]*>/i.test(bodyHtml);

  return (
    <div className="content-pane">
      <div className="content-header">
        <div className="flex items-start justify-between gap-md">
          <h1 className="content-title">{article.title}</h1>

          <div className="flex gap-xs" style={{ flexShrink: 0 }}>
            <button
              className="btn btn-icon btn-ghost"
              onClick={() => onToggleBookmark(article.id)}
              title={article.is_bookmarked ? 'Remove bookmark' : 'Bookmark article'}
            >
              {article.is_bookmarked ? (
                <BookmarkCheck size={18} style={{ color: 'var(--macos-accent)' }} />
              ) : (
                <Bookmark size={18} />
              )}
            </button>

            {article.link && (
              <button
                className="btn btn-icon btn-ghost"
                onClick={() => openUrl(article.link!)}
                title="Open in browser"
              >
                <ExternalLink size={18} />
              </button>
            )}
          </div>
        </div>

        <div className="content-meta">
          {article.author && (
            <div className="flex items-center gap-xs">
              <User size={14} />
              <span>{article.author}</span>
            </div>
          )}
          {article.published_at && (
            <div className="flex items-center gap-xs">
              <Calendar size={14} />
              <span>{formatDate(article.published_at)}</span>
            </div>
          )}
        </div>
      </div>

      <div className="content-body" onClick={handleContentClick}>
        {bodyHtml ? (
          hasHtml ? (
            <div dangerouslySetInnerHTML={{ __html: bodyHtml }} />
          ) : (
            // Plain text — preserve line breaks
            <p style={{ whiteSpace: 'pre-wrap' }}>{bodyHtml}</p>
          )
        ) : (
          <div className="text-secondary">No content available for this article.</div>
        )}

        {article.link && (
          <div className="mt-lg">
            <button className="btn btn-primary" onClick={() => openUrl(article.link!)}>
              Read Original Article
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
