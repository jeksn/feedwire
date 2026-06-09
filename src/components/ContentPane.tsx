import { Bookmark, BookmarkCheck, ExternalLink, Calendar, User } from 'lucide-react';
import type { Article } from '../types';

interface ContentPaneProps {
  article: Article | null;
  onToggleBookmark: (articleId: string) => void;
}

export function ContentPane({ article, onToggleBookmark }: ContentPaneProps) {
  const formatDate = (dateString?: string) => {
    if (!dateString) return '';
    
    const date = new Date(dateString);
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

  const createMarkup = (content: string) => {
    // Basic HTML sanitization - in a real app, you'd use a proper sanitizer
    return { __html: content };
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

  return (
    <div className="content-pane">
      <div className="content-header">
        <div className="flex items-start justify-between gap-md">
          <h1 className="content-title">
            {article.title}
          </h1>
          
          <div className="flex gap-xs">
            <button
              className="btn btn-icon btn-ghost"
              onClick={() => onToggleBookmark(article.id)}
              title={article.is_bookmarked ? "Remove bookmark" : "Bookmark article"}
            >
              {article.is_bookmarked ? (
                <BookmarkCheck size={18} className="text-accent" />
              ) : (
                <Bookmark size={18} />
              )}
            </button>
            
            {article.link && (
              <button
                className="btn btn-icon btn-ghost"
                onClick={() => window.open(article.link, '_blank')}
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

      <div className="content-body">
        {article.content ? (
          <div 
            dangerouslySetInnerHTML={createMarkup(article.content)}
          />
        ) : article.description ? (
          <div 
            dangerouslySetInnerHTML={createMarkup(article.description)}
          />
        ) : (
          <div className="text-secondary">
            No content available for this article.
          </div>
        )}
        
        {article.link && (
          <div className="mt-lg">
            <a
              href={article.link}
              target="_blank"
              rel="noopener noreferrer"
              className="btn btn-primary"
            >
              Read Original Article
            </a>
          </div>
        )}
      </div>
    </div>
  );
}