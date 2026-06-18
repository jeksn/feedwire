import { Bookmark, BookmarkCheck, ExternalLink, Calendar, Rss, Play } from 'lucide-react';
import { openUrl as tauriOpenUrl } from '@tauri-apps/plugin-opener';
import type { Article, Feed } from '../types';

interface ContentPaneProps {
  article: Article | null;
  feeds?: Feed[];
  onToggleBookmark: (articleId: string) => void;
}

function openUrl(url: string) {
  tauriOpenUrl(url).catch(console.error);
}

/** Extract the first <img src> from an HTML string, or null. */
function extractFirstImage(html: string): string | null {
  const m = html.match(/<img[^>]+src=["']([^"']+)["']/i);
  return m ? m[1] : null;
}

export function ContentPane({ article, feeds, onToggleBookmark }: ContentPaneProps) {
  const feed = article ? (feeds?.find(f => f.id === article.feed_id) ?? null) : null;
  const feedName = feed?.title ?? null;

  // YouTube articles have no content body — detect by youtube.com link
  const isYouTube = !!(article?.link?.includes('youtube.com') || article?.link?.includes('youtu.be'));
  const bodyHtml = article ? (article.content || article.description || '') : '';
  const hasBody = bodyHtml.trim().length > 0 && !isYouTube;

  // Pull thumbnail from description HTML if present (YouTube includes it)
  const thumbnail = isYouTube && bodyHtml ? extractFirstImage(bodyHtml) : null;

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

  const hasHtml = /<[a-z][\s\S]*>/i.test(bodyHtml);

  // Only show author if it's different from the feed name (avoids duplication)
  const showAuthor = article.author && article.author !== feedName;

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
          {feedName && (
            <div className="flex items-center gap-xs">
              <Rss size={14} />
              <span>{feedName}</span>
            </div>
          )}
          {article.published_at && (
            <div className="flex items-center gap-xs">
              <Calendar size={14} />
              <span>{formatDate(article.published_at)}</span>
            </div>
          )}
          {showAuthor && (
            <div className="flex items-center gap-xs" style={{ color: 'var(--macos-text-tertiary)' }}>
              <span>{article.author}</span>
            </div>
          )}
        </div>
      </div>

      <div className="content-body" onClick={handleContentClick}>
        {isYouTube ? (
          <div className="yt-card">
            {thumbnail && (
              <div className="yt-thumbnail-wrap" onClick={() => article.link && openUrl(article.link)}>
                <img className="yt-thumbnail" src={thumbnail} alt="" />
                <div className="yt-play-overlay">
                  <Play size={48} fill="white" stroke="none" />
                </div>
              </div>
            )}
            <div className="yt-card-body">
              <p className="yt-card-hint">YouTube videos can't be played inline.</p>
              {article.link && (
                <button className="btn btn-primary yt-watch-btn" onClick={() => openUrl(article.link!)}>
                  <Play size={16} />
                  Watch on YouTube
                </button>
              )}
            </div>
          </div>
        ) : hasBody ? (
          hasHtml ? (
            <div dangerouslySetInnerHTML={{ __html: bodyHtml }} />
          ) : (
            <p style={{ whiteSpace: 'pre-wrap' }}>{bodyHtml}</p>
          )
        ) : (
          <>
            <div className="text-secondary">No content available for this article.</div>
            {article.link && (
              <div className="mt-lg">
                <button className="btn btn-primary" onClick={() => openUrl(article.link!)}>
                  Read Original Article
                </button>
              </div>
            )}
          </>
        )}

        {hasBody && article.link && (
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
