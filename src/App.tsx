import { useState, useEffect } from "react";
import { Sidebar } from "./components/Sidebar";
import { ArticleList } from "./components/ArticleList";
import { ContentPane } from "./components/ContentPane";
import { AddFeedDialog } from "./components/AddFeedDialog";
import { feedApi } from "./api/feed";
import type { Feed, Article } from "./types";
import "./styles/macos.css";

function App() {
  const [feeds, setFeeds] = useState<Feed[]>([]);
  const [articles, setArticles] = useState<Article[]>([]);
  const [selectedFeed, setSelectedFeed] = useState<Feed | null>(null);
  const [selectedArticle, setSelectedArticle] = useState<Article | null>(null);
  const [showAddFeedDialog, setShowAddFeedDialog] = useState(false);
  const [loading, setLoading] = useState(false);
  const [unreadCounts, setUnreadCounts] = useState<Record<string, number>>({});

  useEffect(() => {
    loadFeeds();
    loadArticles();
  }, []);

  const loadFeeds = async () => {
    try {
      const feedsData = await feedApi.getFeeds();
      setFeeds(feedsData);
      
      // Load unread counts for each feed
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

  const handleFeedSelect = (feed: Feed) => {
    setSelectedFeed(feed);
    setSelectedArticle(null);
    loadArticles(feed.id);
  };

  const handleArticleSelect = async (article: Article) => {
    setSelectedArticle(article);
    
    // Mark as read if unread
    if (!article.is_read) {
      try {
        const updatedArticle = await feedApi.markArticleRead(article.id, true);
        setArticles(prev => 
          prev.map(a => a.id === article.id ? updatedArticle : a)
        );
        setSelectedArticle(updatedArticle);
        
        // Update unread count
        if (selectedFeed) {
          setUnreadCounts(prev => ({
            ...prev,
            [selectedFeed.id]: Math.max(0, (prev[selectedFeed.id] || 0) - 1)
          }));
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
      loadFeeds(); // Reload to get updated counts
    } catch (error) {
      console.error("Failed to add feed:", error);
      throw error;
    } finally {
      setLoading(false);
    }
  };

  const handleRefreshFeed = async (feedId: string) => {
    try {
      setLoading(true);
      await feedApi.refreshFeed(feedId);
      loadArticles(feedId);
      loadFeeds(); // Reload to get updated counts
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
      loadArticles();
      loadFeeds();
    } catch (error) {
      console.error("Failed to refresh all feeds:", error);
    } finally {
      setLoading(false);
    }
  };

  const handleToggleBookmark = async (articleId: string) => {
    try {
      const updatedArticle = await feedApi.toggleBookmark(articleId);
      setArticles(prev => 
        prev.map(a => a.id === articleId ? updatedArticle : a)
      );
      if (selectedArticle?.id === articleId) {
        setSelectedArticle(updatedArticle);
      }
    } catch (error) {
      console.error("Failed to toggle bookmark:", error);
    }
  };

  return (
    <div className="app-container">
      <Sidebar
        feeds={feeds}
        selectedFeed={selectedFeed}
        unreadCounts={unreadCounts}
        onFeedSelect={handleFeedSelect}
        onAddFeed={() => setShowAddFeedDialog(true)}
        onRefreshAll={handleRefreshAll}
        loading={loading}
      />
      
      <ArticleList
        articles={articles}
        selectedArticle={selectedArticle}
        onArticleSelect={handleArticleSelect}
        loading={loading}
        selectedFeed={selectedFeed}
        onRefreshFeed={() => selectedFeed && handleRefreshFeed(selectedFeed.id)}
      />
      
      <ContentPane
        article={selectedArticle}
        onToggleBookmark={handleToggleBookmark}
      />
      
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
