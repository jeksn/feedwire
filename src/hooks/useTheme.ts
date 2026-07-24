import { useState, useEffect } from 'react';

export type ThemePreference = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'feedwire-theme';

function applyTheme(pref: ThemePreference) {
  const root = document.documentElement;
  if (pref === 'dark') {
    root.setAttribute('data-theme', 'dark');
  } else if (pref === 'light') {
    root.setAttribute('data-theme', 'light');
  } else {
    // system: follow prefers-color-scheme
    const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    root.setAttribute('data-theme', dark ? 'dark' : 'light');
  }
}

export function useTheme() {
  const [theme, setThemeState] = useState<ThemePreference>(() => {
    return (window.localStorage.getItem(STORAGE_KEY) as ThemePreference) ?? 'system';
  });

  // Apply on mount and whenever preference changes
  useEffect(() => {
    applyTheme(theme);
  }, [theme]);

  // When system is selected, re-apply if the OS theme changes
  useEffect(() => {
    if (theme !== 'system') return;
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const handler = () => applyTheme('system');
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, [theme]);

  const setTheme = (pref: ThemePreference) => {
    window.localStorage.setItem(STORAGE_KEY, pref);
    setThemeState(pref);
  };

  return { theme, setTheme };
}
