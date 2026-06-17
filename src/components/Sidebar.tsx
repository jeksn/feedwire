import { useState, useMemo, useEffect, useRef } from 'react';
import {
  Plus, RefreshCw, Rss, Bookmark, Inbox, Settings,
  ArrowUpDown, ChevronRight, Folder as FolderIcon, Search, X,
} from 'lucide-react';
import type { Feed, Folder } from '../types';
import { feedApi } from '../api/feed';

export type SortOrder = 'alpha-asc' | 'alpha-desc' | 'updated-desc' | 'updated-asc';

const SORT_OPTIONS: { value: SortOrder; label: string }[] = [
  { value: 'alpha-asc',    label: 'A → Z' },
  { value: 'alpha-desc',   label: 'Z → A' },
  { value: 'updated-desc', label: 'Most recent post' },
  { value: 'updated-asc',  label: 'Oldest post' },
];

const SORT_STORAGE_KEY    = 'feedwire-feed-sort';
const COLLAPSE_STORAGE_KEY = 'feedwire-folder-collapsed';

function loadSortOrder(): SortOrder {
  try {
    const stored = localStorage.getItem(SORT_STORAGE_KEY);
    if (stored && SORT_OPTIONS.some(o => o.value === stored)) return stored as SortOrder;
  } catch {}
  return 'alpha-asc';
}

function loadCollapsed(): Set<string> {
  try {
    const stored = localStorage.getItem(COLLAPSE_STORAGE_KEY);
    if (stored) return new Set(JSON.parse(stored) as string[]);
  } catch {}
  return new Set();
}

function saveCollapsed(s: Set<string>) {
  try { localStorage.setItem(COLLAPSE_STORAGE_KEY, JSON.stringify([...s])); } catch {}
}

function sortFeeds(feeds: Feed[], order: SortOrder, unreadCounts: Record<string, number>): Feed[] {
  const copy = [...feeds];

  const byOrder = (a: Feed, b: Feed): number => {
    switch (order) {
      case 'alpha-asc':   return a.title.localeCompare(b.title);
      case 'alpha-desc':  return b.title.localeCompare(a.title);
      case 'updated-desc': {
        const ta = a.latest_article_at ? new Date(a.latest_article_at).getTime() : 0;
        const tb = b.latest_article_at ? new Date(b.latest_article_at).getTime() : 0;
        return tb - ta;
      }
      case 'updated-asc': {
        const ta = a.latest_article_at ? new Date(a.latest_article_at).getTime() : Infinity;
        const tb = b.latest_article_at ? new Date(b.latest_article_at).getTime() : Infinity;
        return ta - tb;
      }
    }
  };

  return copy.sort((a, b) => {
    // Primary: feeds with unread items float to the top
    const aUnread = (unreadCounts[a.id] ?? 0) > 0 ? 0 : 1;
    const bUnread = (unreadCounts[b.id] ?? 0) > 0 ? 0 : 1;
    if (aUnread !== bUnread) return aUnread - bUnread;
    // Tiebreaker: user-chosen sort order
    return byOrder(a, b);
  });
}

// ── Context menu state ───────────────────────────────────────────────────────

interface ContextMenuState {
  feedId: string;
  x: number;
  y: number;
}

interface FolderContextMenuState {
  folder: Folder;
  x: number;
  y: number;
}

// ── Props ────────────────────────────────────────────────────────────────────

interface SidebarProps {
  feeds: Feed[];
  selectedFeed: Feed | null;
  selectedView: 'feed' | 'unread' | 'bookmarks' | 'settings';
  unreadCounts: Record<string, number>;
  onFeedSelect: (feed: Feed) => void;
  onUnreadSelect: () => void;
  onBookmarksSelect: () => void;
  onSettingsSelect: () => void;
  onAddFeed: () => void;
  onRefreshAll: () => void;
  onFeedsChanged: () => void; // called after folder assignment so App re-fetches
  loading: boolean;
}

export function Sidebar({
  feeds,
  selectedFeed,
  selectedView,
  unreadCounts,
  onFeedSelect,
  onUnreadSelect,
  onBookmarksSelect,
  onSettingsSelect,
  onAddFeed,
  onRefreshAll,
  onFeedsChanged,
  loading,
}: SidebarProps) {
  const [sortOrder, setSortOrder] = useState<SortOrder>(loadSortOrder);
  const [collapsed, setCollapsed] = useState<Set<string>>(loadCollapsed);
  const [folders, setFolders] = useState<Folder[]>([]);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const [folderContextMenu, setFolderContextMenu] = useState<FolderContextMenuState | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const searchInputRef = useRef<HTMLInputElement>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);

  const totalUnread = Object.values(unreadCounts).reduce((sum, n) => sum + n, 0);

  // Load folders on mount
  useEffect(() => {
    feedApi.getFolders().then(setFolders).catch(console.error);
  }, [feeds]); // re-fetch when feeds change so folder state stays fresh

  // Close feed context menu on outside click or scroll
  useEffect(() => {
    if (!contextMenu) return;
    const close = () => setContextMenu(null);
    document.addEventListener('mousedown', close);
    document.addEventListener('scroll', close, true);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('scroll', close, true);
    };
  }, [contextMenu]);

  // Close folder context menu on outside click or scroll
  useEffect(() => {
    if (!folderContextMenu) return;
    const close = () => setFolderContextMenu(null);
    document.addEventListener('mousedown', close);
    document.addEventListener('scroll', close, true);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('scroll', close, true);
    };
  }, [folderContextMenu]);

  const handleSortChange = (order: SortOrder) => {
    setSortOrder(order);
    try { localStorage.setItem(SORT_STORAGE_KEY, order); } catch {}
  };

  const toggleCollapse = (folderId: string) => {
    setCollapsed(prev => {
      const next = new Set(prev);
      if (next.has(folderId)) next.delete(folderId);
      else next.add(folderId);
      saveCollapsed(next);
      return next;
    });
  };

  const handleContextMenu = (e: React.MouseEvent, feedId: string) => {
    e.preventDefault();
    e.stopPropagation();
    setFolderContextMenu(null);
    setContextMenu({ feedId, x: e.clientX, y: e.clientY });
  };

  const handleFolderContextMenu = (e: React.MouseEvent, folder: Folder) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu(null);
    setFolderContextMenu({ folder, x: e.clientX, y: e.clientY });
  };

  // Group feeds by folder, applying search filter and sort within each group
  const { ungrouped, grouped, isSearching } = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const isSearching = q.length > 0;
    const filtered = isSearching
      ? feeds.filter(f => f.title.toLowerCase().includes(q))
      : feeds;
    const sorted = sortFeeds(filtered, sortOrder, unreadCounts);
    const ungrouped = sorted.filter(f => !f.folder_id);
    // When searching, flatten everything — no folder grouping
    if (isSearching) {
      return { ungrouped: sorted, grouped: [], isSearching };
    }
    const grouped: { folder: Folder; feeds: Feed[] }[] = folders.map(folder => ({
      folder,
      feeds: sorted.filter(f => f.folder_id === folder.id),
    }));
    return { ungrouped, grouped, isSearching };
  }, [feeds, folders, sortOrder, searchQuery, unreadCounts]);

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
            <span className="feed-unread-count">{totalUnread}</span>
          )}
        </div>
      </div>

      <div className="sidebar-nav">
        <div
          className={`list-item ${selectedView === 'unread' ? 'active' : ''}`}
          onClick={onUnreadSelect}
        >
          <div className="feed-item">
            <div className="flex items-center gap-sm feed-title">
              <Inbox size={14} />
              <span>Unread</span>
            </div>
            {totalUnread > 0 && (
              <span className="feed-unread-count">{totalUnread}</span>
            )}
          </div>
        </div>

        <div
          className={`list-item ${selectedView === 'bookmarks' ? 'active' : ''}`}
          onClick={onBookmarksSelect}
        >
          <div className="feed-item">
            <div className="flex items-center gap-sm feed-title">
              <Bookmark size={14} />
              <span>Bookmarks</span>
            </div>
          </div>
        </div>
      </div>

      <div className="sidebar-section-divider" />

      {feeds.length > 0 && (
        <div className="sidebar-search">
          <Search size={12} className="sidebar-search-icon" />
          <input
            ref={searchInputRef}
            className="sidebar-search-input"
            placeholder="Search feeds…"
            value={searchQuery}
            onChange={e => setSearchQuery(e.target.value)}
            onKeyDown={e => e.key === 'Escape' && setSearchQuery('')}
          />
          {searchQuery && (
            <button
              className="sidebar-search-clear"
              onClick={() => { setSearchQuery(''); searchInputRef.current?.focus(); }}
              title="Clear search"
            >
              <X size={11} />
            </button>
          )}
        </div>
      )}

      {feeds.length > 0 && !isSearching && (
        <div className="sidebar-sort">
          <ArrowUpDown size={11} className="sidebar-sort-icon" />
          <select
            className="sidebar-sort-select"
            value={sortOrder}
            onChange={e => handleSortChange(e.target.value as SortOrder)}
            title="Sort feeds"
          >
            {SORT_OPTIONS.map(opt => (
              <option key={opt.value} value={opt.value}>{opt.label}</option>
            ))}
          </select>
        </div>
      )}

      <div className="list">
        {feeds.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📡</div>
            <div className="empty-state-title">No feeds yet</div>
            <div className="empty-state-description">
              Add your first RSS or Atom feed to get started
            </div>
            <button className="btn btn-primary mt-md" onClick={onAddFeed}>
              Add Feed
            </button>
          </div>
        ) : isSearching && ungrouped.length === 0 ? (
          <div className="sidebar-search-empty">
            No feeds match "{searchQuery}"
          </div>
        ) : (
          <>
            {/* Folders */}
            {grouped.map(({ folder, feeds: folderFeeds }) => (
              <div key={folder.id} className="sidebar-folder-group">
                <button
                  className="sidebar-folder-header"
                  onClick={() => toggleCollapse(folder.id)}
                  onContextMenu={e => handleFolderContextMenu(e, folder)}
                  title={collapsed.has(folder.id) ? 'Expand folder' : 'Collapse folder'}
                >
                  <ChevronRight
                    size={12}
                    className={`sidebar-folder-chevron${collapsed.has(folder.id) ? '' : ' expanded'}`}
                  />
                  <FolderIcon size={13} className="sidebar-folder-icon" />
                  <span className="sidebar-folder-name">{folder.name}</span>
                  {folderFeeds.reduce((s, f) => s + (unreadCounts[f.id] ?? 0), 0) > 0 && (
                    <span className="feed-unread-count">
                      {folderFeeds.reduce((s, f) => s + (unreadCounts[f.id] ?? 0), 0)}
                    </span>
                  )}
                </button>

                {!collapsed.has(folder.id) && folderFeeds.map(feed => (
                  <FeedRow
                    key={feed.id}
                    feed={feed}
                    isActive={selectedView === 'feed' && selectedFeed?.id === feed.id}
                    unreadCount={unreadCounts[feed.id] ?? 0}
                    onSelect={onFeedSelect}
                    onContextMenu={handleContextMenu}
                    indented
                  />
                ))}
              </div>
            ))}

            {/* Ungrouped feeds */}
            {ungrouped.map(feed => (
              <FeedRow
                key={feed.id}
                feed={feed}
                isActive={selectedView === 'feed' && selectedFeed?.id === feed.id}
                unreadCount={unreadCounts[feed.id] ?? 0}
                onSelect={onFeedSelect}
                onContextMenu={handleContextMenu}
                indented={false}
              />
            ))}
          </>
        )}
      </div>

      <div className="sidebar-footer">
        <div className="sidebar-section-divider" />
        <div
          className={`list-item ${selectedView === 'settings' ? 'active' : ''}`}
          onClick={onSettingsSelect}
        >
          <div className="feed-item">
            <div className="flex items-center gap-sm feed-title">
              <Settings size={14} />
              <span>Settings</span>
            </div>
          </div>
        </div>
      </div>

      {/* Feed context menu portal */}
      {contextMenu && (
        <FeedContextMenu
          ref={contextMenuRef}
          feedId={contextMenu.feedId}
          x={contextMenu.x}
          y={contextMenu.y}
          folders={folders}
          currentFolderId={feeds.find(f => f.id === contextMenu.feedId)?.folder_id ?? null}
          onClose={() => setContextMenu(null)}
          onFolderCreated={folder => setFolders(prev => [...prev, folder])}
          onFeedsChanged={onFeedsChanged}
        />
      )}

      {/* Folder context menu portal */}
      {folderContextMenu && (
        <FolderContextMenu
          folder={folderContextMenu.folder}
          x={folderContextMenu.x}
          y={folderContextMenu.y}
          onClose={() => setFolderContextMenu(null)}
          onRenamed={updated => setFolders(prev => prev.map(f => f.id === updated.id ? updated : f))}
          onDeleted={id => {
            setFolders(prev => prev.filter(f => f.id !== id));
            onFeedsChanged();
          }}
        />
      )}
    </div>
  );
}

// ── FeedRow ──────────────────────────────────────────────────────────────────

interface FeedRowProps {
  feed: Feed;
  isActive: boolean;
  unreadCount: number;
  onSelect: (feed: Feed) => void;
  onContextMenu: (e: React.MouseEvent, feedId: string) => void;
  indented: boolean;
}

function FeedRow({ feed, isActive, unreadCount, onSelect, onContextMenu, indented }: FeedRowProps) {
  return (
    <div
      className={`list-item${isActive ? ' active' : ''}${indented ? ' list-item--indented' : ''}`}
      onClick={() => onSelect(feed)}
      onContextMenu={e => onContextMenu(e, feed.id)}
    >
      <div className="feed-item">
        <div className="feed-title truncate">{feed.title}</div>
        {unreadCount > 0 && (
          <span className="feed-unread-count">{unreadCount}</span>
        )}
      </div>
    </div>
  );
}

// ── FeedContextMenu ──────────────────────────────────────────────────────────

interface FeedContextMenuProps {
  feedId: string;
  x: number;
  y: number;
  folders: Folder[];
  currentFolderId: string | null | undefined;
  onClose: () => void;
  onFolderCreated: (folder: Folder) => void;
  onFeedsChanged: () => void;
}

const FeedContextMenu = ({
  feedId, x, y, folders, currentFolderId, onClose, onFolderCreated, onFeedsChanged,
}: FeedContextMenuProps & { ref?: React.Ref<HTMLDivElement> }) => {
  const [creatingFolder, setCreatingFolder] = useState(false);
  const [newFolderName, setNewFolderName] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  // Auto-focus new folder input
  useEffect(() => {
    if (creatingFolder) setTimeout(() => inputRef.current?.focus(), 0);
  }, [creatingFolder]);

  // Clamp position so menu doesn't overflow viewport
  const style = useMemo(() => {
    const menuW = 200;
    const menuH = creatingFolder ? 140 : Math.min(40 + folders.length * 32 + 64, 320);
    const left = Math.min(x, window.innerWidth - menuW - 8);
    const top  = Math.min(y, window.innerHeight - menuH - 8);
    return { left, top };
  }, [x, y, folders.length, creatingFolder]);

  const assign = async (folderId: string | null) => {
    try {
      await feedApi.setFeedFolder(feedId, folderId);
      onFeedsChanged();
    } catch (e) {
      console.error('Failed to assign folder', e);
    }
    onClose();
  };

  const handleNewFolder = async (e: React.FormEvent) => {
    e.preventDefault();
    const name = newFolderName.trim();
    if (!name) return;
    try {
      const folder = await feedApi.createFolder(name);
      onFolderCreated(folder);
      await feedApi.setFeedFolder(feedId, folder.id);
      onFeedsChanged();
    } catch (e) {
      console.error('Failed to create folder', e);
    }
    onClose();
  };

  return (
    <div
      ref={menuRef}
      className="context-menu"
      style={{ position: 'fixed', ...style }}
      onMouseDown={e => e.stopPropagation()} // prevent outside-click handler from firing
    >
      <div className="context-menu-label">Move to folder</div>

      {creatingFolder ? (
        <form className="context-menu-new-folder" onSubmit={handleNewFolder}>
          <input
            ref={inputRef}
            className="context-menu-folder-input"
            placeholder="Folder name…"
            value={newFolderName}
            onChange={e => setNewFolderName(e.target.value)}
            onKeyDown={e => e.key === 'Escape' && onClose()}
          />
          <button type="submit" className="context-menu-item context-menu-item--confirm">
            Create & move
          </button>
          <button
            type="button"
            className="context-menu-item context-menu-item--cancel"
            onClick={() => setCreatingFolder(false)}
          >
            Cancel
          </button>
        </form>
      ) : (
        <>
          {folders.map(folder => (
            <button
              key={folder.id}
              className={`context-menu-item${currentFolderId === folder.id ? ' context-menu-item--active' : ''}`}
              onClick={() => assign(folder.id)}
            >
              <FolderIcon size={12} />
              {folder.name}
              {currentFolderId === folder.id && <span className="context-menu-check">✓</span>}
            </button>
          ))}

          {currentFolderId && (
            <>
              <div className="context-menu-divider" />
              <button className="context-menu-item context-menu-item--remove" onClick={() => assign(null)}>
                Remove from folder
              </button>
            </>
          )}

          <div className="context-menu-divider" />
          <button className="context-menu-item" onClick={() => setCreatingFolder(true)}>
            <Plus size={12} />
            New folder…
          </button>
        </>
      )}
    </div>
  );
};

// ── FolderContextMenu ─────────────────────────────────────────────────────────

interface FolderContextMenuProps {
  folder: Folder;
  x: number;
  y: number;
  onClose: () => void;
  onRenamed: (folder: Folder) => void;
  onDeleted: (folderId: string) => void;
}

function FolderContextMenu({ folder, x, y, onClose, onRenamed, onDeleted }: FolderContextMenuProps) {
  const [mode, setMode] = useState<'menu' | 'rename' | 'delete'>('menu');
  const [renameValue, setRenameValue] = useState(folder.name);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (mode === 'rename') setTimeout(() => { inputRef.current?.focus(); inputRef.current?.select(); }, 0);
  }, [mode]);

  const style = useMemo(() => {
    const menuW = 180;
    const menuH = mode === 'rename' ? 100 : mode === 'delete' ? 120 : 100;
    const left = Math.min(x, window.innerWidth - menuW - 8);
    const top  = Math.min(y, window.innerHeight - menuH - 8);
    return { left, top };
  }, [x, y, mode]);

  const handleRename = async (e: React.FormEvent) => {
    e.preventDefault();
    const name = renameValue.trim();
    if (!name || name === folder.name) { onClose(); return; }
    try {
      const updated = await feedApi.renameFolder(folder.id, name);
      onRenamed(updated);
    } catch (e) {
      console.error('Failed to rename folder', e);
    }
    onClose();
  };

  const handleDelete = async () => {
    try {
      await feedApi.deleteFolder(folder.id);
      onDeleted(folder.id);
    } catch (e) {
      console.error('Failed to delete folder', e);
    }
    onClose();
  };

  return (
    <div
      className="context-menu"
      style={{ position: 'fixed', ...style }}
      onMouseDown={e => e.stopPropagation()}
    >
      {mode === 'menu' && (
        <>
          <div className="context-menu-label">{folder.name}</div>
          <button className="context-menu-item" onClick={() => setMode('rename')}>
            Rename…
          </button>
          <div className="context-menu-divider" />
          <button className="context-menu-item context-menu-item--remove" onClick={() => setMode('delete')}>
            Delete folder…
          </button>
        </>
      )}

      {mode === 'rename' && (
        <form className="context-menu-new-folder" onSubmit={handleRename}>
          <input
            ref={inputRef}
            className="context-menu-folder-input"
            value={renameValue}
            onChange={e => setRenameValue(e.target.value)}
            onKeyDown={e => e.key === 'Escape' && onClose()}
          />
          <button type="submit" className="context-menu-item context-menu-item--confirm">
            Save
          </button>
          <button type="button" className="context-menu-item context-menu-item--cancel" onClick={onClose}>
            Cancel
          </button>
        </form>
      )}

      {mode === 'delete' && (
        <>
          <div className="context-menu-label">Delete "{folder.name}"?</div>
          <div className="context-menu-delete-hint">Feeds will be moved out of this folder.</div>
          <button className="context-menu-item context-menu-item--remove" onClick={handleDelete}>
            Delete folder
          </button>
          <button className="context-menu-item context-menu-item--cancel" onClick={onClose}>
            Cancel
          </button>
        </>
      )}
    </div>
  );
}
