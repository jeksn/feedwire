import { useState, useEffect } from "react";
import { emit } from "@tauri-apps/api/event";
import { Sidebar } from "./components/Sidebar";
import { ArticleList } from "./components/ArticleList";
import { ContentPane } from "./components/ContentPane";
import { AddFeedDialog } from "./components/AddFeedDialog";
import { SettingsPane } from "./components/SettingsPane";
import { feedApi } from "./api/feed";
import { useTheme } from "./hooks/useTheme";
import type { Feed, Article } from "./types";
import "./styles/macos.css";

type View = 'feed' | 'unread' | 'bookmarks' | 'settings';

function App() {
  const { theme, setTheme } = useTheme();
  const [feeds, setFeeds] = useState<Feed[]>([]);
  const [articles, setArticles] = useState<Article[]>([]);
  const [selectedFeed, setSelectedFeed] = useState<Feed | null>(null);
  const [selectedArticle, setSelectedArticle] = useState<Article | null>(null);
  const [selectedView, setSelectedView] = useState<View>('feed');
  const [showAddFeedDialog, setShowAddFeedDialog] = useState(false);
  const [loading, setLoading] = useState(false);
  const [unreadCounts, setUnreadCounts] = useState<Record<string, number>>({});

  // Signal the Rust backend that the React tree has mounted and the window
  // can be made visible. Runs after first paint, so the user never sees a
  // blank frame. Errors are ignored in non-Tauri environments (tests, browser).
  useEffect(() => {
    emit("app-ready").catch(() => {});
  }, []);

  useEffect(() => {
    loadFeeds();
    loadArticles();
  }, []);

  const loadFeeds = async () => {
    try {
      const feedsData = await feedApi.getFeeds();
      setFeeds(feedsData);

      const counts: Record<string, number> = {};
      for (const feed of feedsData) {
        counts[feed.id] = await feedApi.getUnreadCount(feed.id);
      }
      setUnreadCounts(counts);
    } catch (error) {
      console.error("Failed to load feeds:", error);
    }
  };

  const loadArticles = async (feedId?: string) => {
    try {
      setLoading(true);
      const articlesData = await feedApi.getArticles(feedId);
      setArticles(articlesData);
    } catch (error) {
      console.error("Failed to load articles:", error);
    } finally {
      setLoading(false);
    }
  };

  const loadBookmarks = async () => {
    try {
      setLoading(true);
      const bookmarked = await feedApi.getBookmarkedArticles();
      setArticles(bookmarked);
    } catch (error) {
      console.error("Failed to load bookmarks:", error);
    } finally {
      setLoading(false);
    }
  };

  const loadUnread = async () => {
    try {
      setLoading(true);
      const unread = await feedApi.getUnreadArticles();
      setArticles(unread);
    } catch (error) {
      console.error("Failed to load unread:", error);
    } finally {
      setLoading(false);
    }
  };

  const handleFeedSelect = (feed: Feed) => {
    setSelectedFeed(feed);
    setSelectedArticle(null);
    setSelectedView('feed');
    loadArticles(feed.id);
  };

  const handleUnreadSelect = () => {
    setSelectedFeed(null);
    setSelectedArticle(null);
    setSelectedView('unread');
    loadUnread();
  };

  const handleBookmarksSelect = () => {
    setSelectedFeed(null);
    setSelectedArticle(null);
    setSelectedView('bookmarks');
    loadBookmarks();
  };

  const handleSettingsSelect = () => {
    setSelectedFeed(null);
    setSelectedArticle(null);
    setSelectedView('settings');
  };

  const handleImportOpml = async (): Promise<string> => {
    const result = await feedApi.importOpml();
    // Refresh feeds list after import
    await loadFeeds();
    if (selectedView === 'feed') loadArticles(selectedFeed?.id);
    return result;
  };

  const handleExportOpml = async (): Promise<string> => {
    return await feedApi.exportOpml();
  };

  const handleArticleSelect = async (article: Article) => {
    setSelectedArticle(article);

    if (!article.is_read) {
      try {
        const updatedArticle = await feedApi.markArticleRead(article.id, true);
        // In unread view, keep article visible while selected but update its state
        setArticles(prev => prev.map(a => a.id === article.id ? updatedArticle : a));
        setSelectedArticle(updatedArticle);

        if (selectedFeed) {
          setUnreadCounts(prev => ({
            ...prev,
            [selectedFeed.id]: Math.max(0, (prev[selectedFeed.id] || 0) - 1)
          }));
        } else {
          // Update global unread counts even without a selected feed
          loadFeeds();
        }
      } catch (error) {
        console.error("Failed to mark article as read:", error);
      }
    }
  };

  const handleAddFeed = async (url: string) => {
    try {
      setLoading(true);
      const newFeed = await feedApi.addFeed(url);
      setFeeds(prev => [...prev, newFeed]);
      setShowAddFeedDialog(false);
      // Auto-select the new feed and load its articles
      setSelectedFeed(newFeed);
      setSelectedView('feed');
      await loadFeeds();
      await loadArticles(newFeed.id);
    } catch (error) {
      console.error("Failed to add feed:", error);
      throw error;
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteFeed = async (feedId: string) => {
    try {
      await feedApi.deleteFeed(feedId);
      setFeeds(prev => prev.filter(f => f.id !== feedId));
      setUnreadCounts(prev => {
        const next = { ...prev };
        delete next[feedId];
        return next;
      });
      if (selectedFeed?.id === feedId) {
        setSelectedFeed(null);
        setSelectedView('feed');
        setArticles([]);
        setSelectedArticle(null);
      }
    } catch (error) {
      console.error("Failed to delete feed:", error);
    }
  };

  const handleRefreshFeed = async (feedId: string) => {
    try {
      setLoading(true);
      await feedApi.refreshFeed(feedId);
      loadArticles(feedId);
      loadFeeds();
    } catch (error) {
      console.error("Failed to refresh feed:", error);
    } finally {
      setLoading(false);
    }
  };

  const handleRefreshAll = async () => {
    try {
      setLoading(true);
      await feedApi.refreshAllFeeds();
      if (selectedView === 'bookmarks') {
        loadBookmarks();
      } else if (selectedView === 'unread') {
        loadUnread();
      } else {
        loadArticles(selectedFeed?.id);
      }
      loadFeeds();
    } catch (error) {
      console.error("Failed to refresh all feeds:", error);
    } finally {
      setLoading(false);
    }
  };

  const handleMarkAllRead = async () => {
    try {
      // Pass feedId when in a single feed view, undefined marks all globally
      await feedApi.markAllRead(selectedView === 'feed' ? selectedFeed?.id : undefined);
      // Update article list in place — mark everything shown as read
      setArticles(prev => prev.map(a => ({ ...a, is_read: true })));
      if (selectedArticle) setSelectedArticle(prev => prev ? { ...prev, is_read: true } : null);
      // Zero out unread counts
      if (selectedView === 'feed' && selectedFeed) {
        setUnreadCounts(prev => ({ ...prev, [selectedFeed.id]: 0 }));
      } else {
        setUnreadCounts(prev => Object.fromEntries(Object.keys(prev).map(k => [k, 0])));
      }
    } catch (error) {
      console.error("Failed to mark all as read:", error);
    }
  };

  const handleToggleBookmark = async (articleId: string) => {
    try {
      const updatedArticle = await feedApi.toggleBookmark(articleId);
      setArticles(prev => {
        // In bookmarks view, remove article if it's been unbookmarked
        if (selectedView === 'bookmarks' && !updatedArticle.is_bookmarked) {
          return prev.filter(a => a.id !== articleId);
        }
        return prev.map(a => a.id === articleId ? updatedArticle : a);
      });
      if (selectedArticle?.id === articleId) {
        // If unbookmarked while in bookmarks view, deselect
        if (selectedView === 'bookmarks' && !updatedArticle.is_bookmarked) {
          setSelectedArticle(null);
        } else {
          setSelectedArticle(updatedArticle);
        }
      }
    } catch (error) {
      console.error("Failed to toggle bookmark:", error);
    }
  };

  const articleListTitle = selectedView === 'bookmarks' ? 'Bookmarks'
    : selectedView === 'unread' ? 'Unread' : undefined;

  return (
    <div className="app-container">
      <Sidebar
        feeds={feeds}
        selectedFeed={selectedFeed}
        selectedView={selectedView}
        unreadCounts={unreadCounts}
        onFeedSelect={handleFeedSelect}
        onUnreadSelect={handleUnreadSelect}
        onBookmarksSelect={handleBookmarksSelect}
        onSettingsSelect={handleSettingsSelect}
        onAddFeed={() => setShowAddFeedDialog(true)}
        onRefreshAll={handleRefreshAll}
        loading={loading}
      />

      {selectedView === 'settings' ? (
        <SettingsPane
          feedCount={feeds.length}
          onImport={handleImportOpml}
          onExport={handleExportOpml}
          theme={theme}
          onThemeChange={setTheme}
        />
      ) : (
        <>
          <ArticleList
            articles={articles}
            selectedArticle={selectedArticle}
            onArticleSelect={handleArticleSelect}
            loading={loading}
            selectedFeed={selectedFeed}
            title={articleListTitle}
            onRefreshFeed={() => selectedFeed && handleRefreshFeed(selectedFeed.id)}
            onMarkAllRead={selectedView !== 'bookmarks' ? handleMarkAllRead : undefined}
            onDeleteFeed={selectedView === 'feed' && selectedFeed ? () => handleDeleteFeed(selectedFeed.id) : undefined}
          />

          <ContentPane
            article={selectedArticle}
            onToggleBookmark={handleToggleBookmark}
          />
        </>
      )}

      {showAddFeedDialog && (
        <AddFeedDialog
          onClose={() => setShowAddFeedDialog(false)}
          onAddFeed={handleAddFeed}
        />
      )}
    </div>
  );
}

export default App;
