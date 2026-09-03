import { useCallback, useEffect, useRef, useState } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export type UpdateStatus = 'idle' | 'checking' | 'available' | 'downloading' | 'up-to-date' | 'error';

/**
 * In-app updates via the Tauri updater plugin. Checks GitHub releases
 * (latest.json) silently shortly after launch, and exposes a manual check +
 * download-and-install flow for the Settings pane and the sidebar pill.
 */
export function useUpdater() {
  const [status, setStatus] = useState<UpdateStatus>('idle');
  const [progress, setProgress] = useState<number | null>(null);
  const updateRef = useRef<Update | null>(null);

  const checkForUpdate = useCallback(async (): Promise<boolean> => {
    setStatus('checking');
    try {
      const update = await check();
      updateRef.current = update ?? null;
      if (update) {
        setStatus('available');
        return true;
      }
      setStatus('up-to-date');
      return false;
    } catch (e) {
      console.error('Update check failed:', e);
      setStatus('error');
      return false;
    }
  }, []);

  const installUpdate = useCallback(async () => {
    const update = updateRef.current;
    if (!update) return;
    setStatus('downloading');
    setProgress(0);
    try {
      let total = 0;
      let received = 0;
      await update.downloadAndInstall(event => {
        switch (event.event) {
          case 'Started':
            total = event.data.contentLength ?? 0;
            break;
          case 'Progress':
            received += event.data.chunkLength;
            if (total > 0) setProgress(Math.min(100, Math.round((received / total) * 100)));
            break;
          case 'Finished':
            setProgress(100);
            break;
        }
      });
      await relaunch();
    } catch (e) {
      console.error('Update install failed:', e);
      setStatus('available');
      setProgress(null);
    }
  }, []);

  // Silent check shortly after launch. Errors are ignored — the app works
  // fine offline and there may be no releases published yet (404).
  useEffect(() => {
    const t = setTimeout(() => {
      checkForUpdate().catch(() => {});
    }, 15000);
    return () => clearTimeout(t);
  }, [checkForUpdate]);

  return { status, progress, checkForUpdate, installUpdate };
}
