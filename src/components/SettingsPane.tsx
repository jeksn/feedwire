import { useState, useEffect, useCallback } from 'react';
import { Upload, Download, CheckCircle, AlertCircle, Loader, Sun, Moon, Monitor, ChevronDown, ChevronUp, Trash2, Plus, X, RefreshCw } from 'lucide-react';
import type { ThemePreference } from '../hooks/useTheme';
import type { ImportResult } from '../api/feed';
import { feedApi } from '../api/feed';
import type { FilterRule, FilterField, FilterSettings } from '../types';

interface SettingsPaneProps {
  feedCount: number;
  onImport: () => Promise<ImportResult>;
  onExport: () => Promise<string>;
  onDeleteAll: () => Promise<number>;
  theme: ThemePreference;
  onThemeChange: (t: ThemePreference) => void;
}

type ImportStatus =
  | { type: 'idle' }
  | { type: 'loading' }
  | { type: 'done'; result: ImportResult }
  | { type: 'error'; message: string };

type ExportStatus =
  | { type: 'idle' }
  | { type: 'loading' }
  | { type: 'success'; message: string }
  | { type: 'error'; message: string };

type DeleteAllStatus = { type: 'idle' } | { type: 'confirm' } | { type: 'loading' } | { type: 'done'; count: number };

export function SettingsPane({ feedCount, onImport, onExport, onDeleteAll, theme, onThemeChange }: SettingsPaneProps) {
  const [importStatus, setImportStatus] = useState<ImportStatus>({ type: 'idle' });
  const [exportStatus, setExportStatus] = useState<ExportStatus>({ type: 'idle' });
  const [deleteAllStatus, setDeleteAllStatus] = useState<DeleteAllStatus>({ type: 'idle' });

  const handleImport = async () => {
    setImportStatus({ type: 'loading' });
    try {
      const result = await onImport();
      setImportStatus({ type: 'done', result });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.toLowerCase().includes('cancel')) {
        setImportStatus({ type: 'idle' });
      } else {
        setImportStatus({ type: 'error', message: msg });
      }
    }
  };

  const handleDeleteAll = async () => {
    if (deleteAllStatus.type !== 'confirm') {
      setDeleteAllStatus({ type: 'confirm' });
      return;
    }
    setDeleteAllStatus({ type: 'loading' });
    try {
      const count = await onDeleteAll();
      setDeleteAllStatus({ type: 'done', count });
    } catch (err) {
      // Reset on error — unlikely but safe
      setDeleteAllStatus({ type: 'idle' });
    }
  };

  const handleExport = async () => {
    setExportStatus({ type: 'loading' });
    try {
      const message = await onExport();
      setExportStatus({ type: 'success', message });
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      if (msg.toLowerCase().includes('cancel')) {
        setExportStatus({ type: 'idle' });
      } else {
        setExportStatus({ type: 'error', message: msg });
      }
    }
  };

  return (
    <div className="settings-pane">
      <div className="settings-header">
        <h2 className="settings-title">Settings</h2>
      </div>

      <div className="settings-scroll">
      <div className="settings-body">
        {/* Appearance section */}
        <section className="settings-section">
          <h3 className="settings-section-title">Appearance</h3>
          <div className="theme-segment">
            <button
              className={`theme-segment-btn${theme === 'light' ? ' active' : ''}`}
              onClick={() => onThemeChange('light')}
            >
              <Sun size={14} />
              Light
            </button>
            <button
              className={`theme-segment-btn${theme === 'system' ? ' active' : ''}`}
              onClick={() => onThemeChange('system')}
            >
              <Monitor size={14} />
              System
            </button>
            <button
              className={`theme-segment-btn${theme === 'dark' ? ' active' : ''}`}
              onClick={() => onThemeChange('dark')}
            >
              <Moon size={14} />
              Dark
            </button>
          </div>
        </section>

        {/* Auto-refresh section */}
        <AutoRefreshSection />

        {/* Import / Export section */}
        <section className="settings-section">
          <h3 className="settings-section-title">Import & Export</h3>
          <p className="settings-section-description">
            Use OPML to transfer your subscriptions between RSS readers. OPML is
            supported by NetNewsWire, Reeder, Feedly, Miniflux, and virtually all
            other feed readers.
          </p>

          <div className="settings-actions">
            {/* Import */}
            <div className="settings-action-card">
              <div className="settings-action-info">
                <div className="flex items-center gap-sm">
                  <Upload size={18} className="text-accent" />
                  <span className="settings-action-title">Import OPML</span>
                </div>
                <p className="settings-action-description">
                  Load subscriptions from an .opml file. Feeds already in your
                  library are left untouched.
                </p>
              </div>
              <button
                className="btn btn-secondary settings-action-btn"
                onClick={handleImport}
                disabled={importStatus.type === 'loading'}
              >
                {importStatus.type === 'loading' ? (
                  <Loader size={14} className="animate-spin" />
                ) : (
                  <Upload size={14} />
                )}
                Import
              </button>
              <ImportStatusDisplay status={importStatus} />
            </div>

            {/* Export */}
            <div className="settings-action-card">
              <div className="settings-action-info">
                <div className="flex items-center gap-sm">
                  <Download size={18} className="text-accent" />
                  <span className="settings-action-title">Export OPML</span>
                </div>
                <p className="settings-action-description">
                  Save all {feedCount} {feedCount === 1 ? 'feed' : 'feeds'} to an
                  .opml file you can import into any other reader.
                </p>
              </div>
              <button
                className="btn btn-secondary settings-action-btn"
                onClick={handleExport}
                disabled={exportStatus.type === 'loading' || feedCount === 0}
              >
                {exportStatus.type === 'loading' ? (
                  <Loader size={14} className="animate-spin" />
                ) : (
                  <Download size={14} />
                )}
                Export
              </button>
              <ExportStatusBadge status={exportStatus} />
            </div>
          </div>
        </section>

        {/* Data section */}
        <section className="settings-section">
          <h3 className="settings-section-title">Data</h3>
          <p className="settings-section-description">
            Manage your stored feed data. These actions cannot be undone.
          </p>
          <div className="settings-actions">
            <div className="settings-action-card settings-action-card--danger">
              <div className="settings-action-info">
                <div className="flex items-center gap-sm">
                  <Trash2 size={18} className="text-destructive" />
                  <span className="settings-action-title">Delete All Feeds</span>
                </div>
                <p className="settings-action-description">
                  Permanently removes all {feedCount} {feedCount === 1 ? 'feed' : 'feeds'} and
                  their articles. This cannot be undone.
                </p>
              </div>
              {deleteAllStatus.type === 'done' ? (
                <div className="settings-status settings-status-success">
                  <CheckCircle size={13} />
                  <span>Removed {deleteAllStatus.count} {deleteAllStatus.count === 1 ? 'feed' : 'feeds'}</span>
                </div>
              ) : (
                <div className="delete-all-actions">
                  {deleteAllStatus.type === 'confirm' && (
                    <button
                      className="btn btn-ghost settings-action-btn"
                      onClick={() => setDeleteAllStatus({ type: 'idle' })}
                    >
                      Cancel
                    </button>
                  )}
                  <button
                    className={`btn settings-action-btn ${deleteAllStatus.type === 'confirm' ? 'btn-destructive' : 'btn-destructive-outline'}`}
                    onClick={handleDeleteAll}
                    disabled={deleteAllStatus.type === 'loading' || feedCount === 0}
                  >
                    {deleteAllStatus.type === 'loading' ? (
                      <Loader size={14} className="animate-spin" />
                    ) : (
                      <Trash2 size={14} />
                    )}
                    {deleteAllStatus.type === 'confirm' ? 'Yes, delete everything' : 'Delete All Feeds'}
                  </button>
                </div>
              )}
            </div>
          </div>
        </section>

        {/* Filter rules section */}
        <FilterRulesSection />

        {/* About section */}
        <section className="settings-section">
          <h3 className="settings-section-title">About</h3>
          <div className="settings-about">
            <p className="text-sm text-secondary">FeedWire — a minimal RSS reader</p>
            <p className="text-xs text-secondary" style={{ marginTop: 4 }}>Version 0.1.0</p>
          </div>
        </section>
      </div>
      </div>
    </div>
  );
}

function ImportStatusDisplay({ status }: { status: ImportStatus }) {
  const [failuresExpanded, setFailuresExpanded] = useState(false);

  if (status.type === 'idle' || status.type === 'loading') return null;

  if (status.type === 'error') {
    return (
      <div className="settings-status settings-status-error">
        <AlertCircle size={13} />
        <span>{status.message}</span>
      </div>
    );
  }

  const { added, skipped, failed } = status.result;
  const hasFailures = failed.length > 0;
  const allFailed = added === 0 && skipped === 0 && hasFailures;

  return (
    <div className="import-result">
      {/* Summary row */}
      <div className={`import-result-summary ${allFailed ? 'import-result-summary--error' : 'import-result-summary--success'}`}>
        {allFailed ? <AlertCircle size={13} /> : <CheckCircle size={13} />}
        <div className="import-result-counts">
          {added > 0 && (
            <span className="import-count import-count--added">{added} added</span>
          )}
          {skipped > 0 && (
            <span className="import-count import-count--skipped">{skipped} already in library</span>
          )}
          {hasFailures && (
            <span className="import-count import-count--failed">{failed.length} failed</span>
          )}
        </div>
        {hasFailures && (
          <button
            className="import-failures-toggle"
            onClick={() => setFailuresExpanded(v => !v)}
            aria-expanded={failuresExpanded}
          >
            {failuresExpanded ? <ChevronUp size={12} /> : <ChevronDown size={12} />}
            {failuresExpanded ? 'Hide' : 'Show'} failures
          </button>
        )}
      </div>

      {/* Failures list */}
      {hasFailures && failuresExpanded && (
        <ul className="import-failures-list">
          {failed.map((f, i) => (
            <li key={i} className="import-failure-item">
              <div className="import-failure-title">{f.title || f.url}</div>
              <div className="import-failure-url">{f.url}</div>
              <div className="import-failure-reason">{f.reason}</div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function ExportStatusBadge({ status }: { status: ExportStatus }) {
  if (status.type === 'idle' || status.type === 'loading') return null;

  if (status.type === 'success') {
    return (
      <div className="settings-status settings-status-success">
        <CheckCircle size={13} />
        <span>{status.message}</span>
      </div>
    );
  }

  return (
    <div className="settings-status settings-status-error">
      <AlertCircle size={13} />
      <span>{status.message}</span>
    </div>
  );
}

// ── Auto-refresh Section ──────────────────────────────────────────────────────

const INTERVAL_OPTIONS: { label: string; value: number }[] = [
  { label: 'Off',        value: 0   },
  { label: 'Every 30 min', value: 30  },
  { label: 'Every hour',   value: 60  },
  { label: 'Every 2 hours', value: 120 },
];

function AutoRefreshSection() {
  const [interval, setInterval] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    feedApi.getAutoRefreshInterval()
      .then(v => setInterval(v))
      .catch(console.error);
  }, []);

  const handleChange = async (minutes: number) => {
    setInterval(minutes);
    setSaving(true);
    try {
      await feedApi.setAutoRefreshInterval(minutes);
    } catch (e) {
      console.error('Failed to save auto-refresh interval', e);
    } finally {
      setSaving(false);
    }
  };

  return (
    <section className="settings-section">
      <h3 className="settings-section-title">Refresh</h3>
      <p className="settings-section-description">
        Feeds are refreshed automatically on launch. You can also set a
        background refresh interval so new articles appear while the app
        is open.
      </p>

      <div className="settings-row">
        <div className="settings-row-label">
          <RefreshCw size={14} className="text-accent" />
          <span>Background refresh</span>
        </div>
        <div className="settings-row-control">
          {interval === null ? (
            <Loader size={13} className="animate-spin" />
          ) : (
            <div className="auto-refresh-segment">
              {INTERVAL_OPTIONS.map(opt => (
                <button
                  key={opt.value}
                  className={`auto-refresh-btn${interval === opt.value ? ' active' : ''}`}
                  onClick={() => handleChange(opt.value)}
                  disabled={saving}
                >
                  {opt.label}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>
    </section>
  );
}

// ── Filter Rules Section ──────────────────────────────────────────────────────

function FilterRulesSection() {
  const [settings, setSettings] = useState<FilterSettings | null>(null);
  const [loading, setLoading] = useState(true);
  const [newPattern, setNewPattern] = useState('');
  const [newField, setNewField] = useState<FilterField>('url');
  const [addError, setAddError] = useState('');

  const load = useCallback(async () => {
    try {
      const s = await feedApi.getFilterSettings();
      setSettings(s);
    } catch (e) {
      console.error('Failed to load filter settings', e);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { load(); }, [load]);

  const handleShortsToggle = async () => {
    if (!settings) return;
    const next = !settings.skip_youtube_shorts;
    setSettings(s => s ? { ...s, skip_youtube_shorts: next } : s);
    try {
      await feedApi.setSkipYoutubeShorts(next);
    } catch (e) {
      // Revert on failure
      setSettings(s => s ? { ...s, skip_youtube_shorts: !next } : s);
    }
  };

  const handleAddRule = async (e: React.FormEvent) => {
    e.preventDefault();
    const pattern = newPattern.trim();
    if (!pattern) { setAddError('Pattern cannot be empty'); return; }
    setAddError('');
    try {
      const rule = await feedApi.addFilterRule(pattern, newField);
      setSettings(s => s ? { ...s, rules: [...s.rules, rule] } : s);
      setNewPattern('');
    } catch (e) {
      setAddError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleToggleRule = async (rule: FilterRule) => {
    const next = !rule.enabled;
    setSettings(s => s ? { ...s, rules: s.rules.map(r => r.id === rule.id ? { ...r, enabled: next } : r) } : s);
    try {
      await feedApi.updateFilterRuleEnabled(rule.id, next);
    } catch {
      // Revert
      setSettings(s => s ? { ...s, rules: s.rules.map(r => r.id === rule.id ? { ...r, enabled: !next } : r) } : s);
    }
  };

  const handleDeleteRule = async (ruleId: string) => {
    setSettings(s => s ? { ...s, rules: s.rules.filter(r => r.id !== ruleId) } : s);
    try {
      await feedApi.deleteFilterRule(ruleId);
    } catch {
      // Reload to restore state
      load();
    }
  };

  return (
    <section className="settings-section">
      <h3 className="settings-section-title">Article Filters</h3>
      <p className="settings-section-description">
        Articles matching any enabled filter are hidden when feeds are fetched or
        refreshed. Existing articles are not affected retroactively.
      </p>

      {loading ? (
        <div className="filter-loading"><Loader size={14} className="animate-spin" /> Loading…</div>
      ) : settings && (
        <>
          {/* YouTube Shorts toggle */}
          <div className="filter-toggle-row" onClick={handleShortsToggle} role="button" tabIndex={0}
            onKeyDown={e => e.key === 'Enter' && handleShortsToggle()}>
            <div className="filter-toggle-info">
              <span className="filter-toggle-label">Skip YouTube Shorts</span>
              <span className="filter-toggle-description">
                Hide articles whose URL contains <code>youtube.com/shorts/</code>
              </span>
            </div>
            <div className={`filter-toggle${settings.skip_youtube_shorts ? ' filter-toggle--on' : ''}`}>
              <div className="filter-toggle-thumb" />
            </div>
          </div>

          {/* User-defined rules */}
          <div className="filter-rules-list">
            {settings.rules.length === 0 ? (
              <p className="filter-rules-empty">No custom rules yet.</p>
            ) : (
              settings.rules.map(rule => (
                <div key={rule.id} className={`filter-rule-row${rule.enabled ? '' : ' filter-rule-row--disabled'}`}>
                  <button
                    className={`filter-rule-check${rule.enabled ? ' filter-rule-check--on' : ''}`}
                    onClick={() => handleToggleRule(rule)}
                    title={rule.enabled ? 'Disable rule' : 'Enable rule'}
                  />
                  <span className="filter-rule-field">{rule.field}</span>
                  <span className="filter-rule-pattern">{rule.pattern}</span>
                  <button
                    className="filter-rule-delete"
                    onClick={() => handleDeleteRule(rule.id)}
                    title="Delete rule"
                  >
                    <X size={12} />
                  </button>
                </div>
              ))
            )}
          </div>

          {/* Add rule form */}
          <form className="filter-add-form" onSubmit={handleAddRule}>
            <select
              className="filter-add-field"
              value={newField}
              onChange={e => setNewField(e.target.value as FilterField)}
            >
              <option value="url">URL</option>
              <option value="title">Title</option>
            </select>
            <input
              className="filter-add-input"
              placeholder="Substring to match…"
              value={newPattern}
              onChange={e => { setNewPattern(e.target.value); setAddError(''); }}
            />
            <button className="btn btn-secondary filter-add-btn" type="submit">
              <Plus size={13} /> Add
            </button>
          </form>
          {addError && (
            <div className="settings-status settings-status-error" style={{ marginTop: 6 }}>
              <AlertCircle size={13} /><span>{addError}</span>
            </div>
          )}
        </>
      )}
    </section>
  );
}
