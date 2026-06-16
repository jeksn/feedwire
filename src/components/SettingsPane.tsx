import { useState } from 'react';
import { Upload, Download, CheckCircle, AlertCircle, Loader, Sun, Moon, Monitor, ChevronDown, ChevronUp } from 'lucide-react';
import type { ThemePreference } from '../hooks/useTheme';
import type { ImportResult } from '../api/feed';

interface SettingsPaneProps {
  feedCount: number;
  onImport: () => Promise<ImportResult>;
  onExport: () => Promise<string>;
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

export function SettingsPane({ feedCount, onImport, onExport, theme, onThemeChange }: SettingsPaneProps) {
  const [importStatus, setImportStatus] = useState<ImportStatus>({ type: 'idle' });
  const [exportStatus, setExportStatus] = useState<ExportStatus>({ type: 'idle' });

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
                  Load subscriptions from an .opml file. Existing feeds are skipped
                  automatically.
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
            <span className="import-count import-count--skipped">{skipped} skipped</span>
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
