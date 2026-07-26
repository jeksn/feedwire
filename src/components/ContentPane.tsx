import { useEffect, useState } from 'react';
import { Bookmark, BookmarkCheck, ExternalLink, Calendar, Rss } from 'lucide-react';
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

/** Tiny YouTube logo SVG (lucide-react version doesn't include it). */
function YouTubeIcon({ size = 14 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="currentColor"
      aria-hidden="true"
    >
      <path d="M23.498 6.186a2.996 2.996 0 0 0-2.116-2.116C19.724 3.5 12 3.5 12 3.5s-7.724 0-9.382.57A2.996 2.996 0 0 0 .502 6.186C0 7.844 0 12 0 12s0 4.156.502 5.814a2.996 2.996 0 0 0 2.116 2.116c1.658.57 9.382.57 9.382.57s7.724 0 9.382-.57a2.996 2.996 0 0 0 2.116-2.116C24 16.156 24 12 24 12s0-4.156-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z" />
    </svg>
  );
}

/** Extract the first <img src> from an HTML string, or null. */
function extractFirstImage(html: string): string | null {
  const m = html.match(/<img[^>]+src=["']([^"']+)["']/i);
  return m ? m[1] : null;
}

function youtubeThumbnailUrl(link: string | undefined, size: 'maxresdefault' | 'hqdefault'): string | null {
  if (!link) return null;
  try {
    const url = new URL(link);
    const videoId = url.hostname.endsWith('youtu.be')
      ? url.pathname.split('/').filter(Boolean)[0]
      : url.searchParams.get('v');
    return videoId ? `https://i.ytimg.com/vi/${videoId}/${size}.jpg` : null;
  } catch {
    return null;
  }
}

export function ContentPane({ article, feeds, onToggleBookmark }: ContentPaneProps) {
  const [hoveredLink, setHoveredLink] = useState<string | null>(null);
  const [highResolutionThumbnailFailed, setHighResolutionThumbnailFailed] = useState(false);

  useEffect(() => {
    setHighResolutionThumbnailFailed(false);
  }, [article?.id]);

  const feed = article ? (feeds?.find(f => f.id === article.feed_id) ?? null) : null;
  const feedName = feed?.title ?? null;
  const isYouTube = !!(feed?.url?.includes('youtube.com') || feed?.url?.includes('youtu.be'));

  const bodyHtml = article ? (article.content || article.description || '') : '';
  const hasBody = bodyHtml.trim().length > 0 && !isYouTube;

  const fallbackThumbnail = article?.thumbnail_url
    ?? (bodyHtml ? extractFirstImage(bodyHtml) : null)
    ?? youtubeThumbnailUrl(article?.link, 'hqdefault');
  const highResolutionThumbnail = youtubeThumbnailUrl(article?.link, 'maxresdefault');
  const thumbnail = isYouTube
    ? (highResolutionThumbnailFailed ? fallbackThumbnail : highResolutionThumbnail ?? fallbackThumbnail)
    : null;

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

  // Suppress the author when it's just the channel name (case-insensitive)
  const showAuthor = article.author &&
    article.author.trim().toLowerCase() !== (feedName ?? '').trim().toLowerCase();

  return (
    <div
      className="content-pane"
      onMouseOver={(e) => {
        const anchor = (e.target as HTMLElement).closest<HTMLAnchorElement>('a[href]');
        if (anchor?.href) setHoveredLink(anchor.href);
      }}
      onMouseOut={(e) => {
        const anchor = (e.target as HTMLElement).closest<HTMLAnchorElement>('a[href]');
        if (anchor) setHoveredLink(null);
      }}
    >
      <div className="content-header">
        <div className="flex items-start justify-between gap-md">
          <h1 className="content-title">
            {article.link ? (
              <a
                className="content-title-link"
                href={article.link}
                onClick={(e) => {
                  e.preventDefault();
                  openUrl(article.link!);
                }}
                title="Open in browser"
              >
                {article.title}
                <ExternalLink className="content-title-external-link" size={16} aria-hidden="true" />
              </a>
            ) : (
              article.title
            )}
          </h1>

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

          </div>
        </div>

        <div className="content-meta">
          {feedName && (
            <div className="flex items-center gap-xs">
              {isYouTube ? <YouTubeIcon size={14} /> : <Rss size={14} />}
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
          thumbnail && (
            <div className="yt-card">
              <div className="yt-thumbnail-wrap" onClick={() => article.link && openUrl(article.link)}>
                <img
                  className="yt-thumbnail"
                  src={thumbnail}
                  alt=""
                  onError={() => setHighResolutionThumbnailFailed(true)}
                />
                <div className="yt-play-overlay">
                  <ExternalLink size={40} />
                </div>
              </div>
            </div>
          )
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

      {hoveredLink && (
        <div className="content-status-bar" aria-hidden="true">
          {hoveredLink}
        </div>
      )}
    </div>
  );
}
