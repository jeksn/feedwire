import { renderHook, act } from '@testing-library/react';
import { useTheme } from './useTheme';

// jsdom doesn't implement matchMedia — provide a minimal stub
function mockMatchMedia(prefersDark: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn((query: string) => ({
      matches: query.includes('dark') ? prefersDark : !prefersDark,
      media: query,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    })),
  });
}

beforeEach(() => {
  window.localStorage.clear();
  mockMatchMedia(false); // default: light OS theme
  // Reset data-theme attribute
  document.documentElement.removeAttribute('data-theme');
});

describe('useTheme', () => {
  it('defaults to system when no preference stored', () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe('system');
  });

  it('restores persisted preference from localStorage', () => {
    window.localStorage.setItem('feedwire-theme', 'dark');
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBe('dark');
  });

  it('applies data-theme="dark" when set to dark', () => {
    const { result } = renderHook(() => useTheme());
    act(() => result.current.setTheme('dark'));
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('applies data-theme="light" when set to light', () => {
    const { result } = renderHook(() => useTheme());
    act(() => result.current.setTheme('light'));
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  it('applies data-theme="light" for system when OS is light', () => {
    mockMatchMedia(false); // OS = light
    const { result } = renderHook(() => useTheme());
    act(() => result.current.setTheme('system'));
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
  });

  it('applies data-theme="dark" for system when OS is dark', () => {
    mockMatchMedia(true); // OS = dark
    const { result } = renderHook(() => useTheme());
    act(() => result.current.setTheme('system'));
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('persists preference to localStorage', () => {
    const { result } = renderHook(() => useTheme());
    act(() => result.current.setTheme('dark'));
    expect(window.localStorage.getItem('feedwire-theme')).toBe('dark');
  });

  it('updates theme state when setTheme is called', () => {
    const { result } = renderHook(() => useTheme());
    act(() => result.current.setTheme('dark'));
    expect(result.current.theme).toBe('dark');
    act(() => result.current.setTheme('light'));
    expect(result.current.theme).toBe('light');
  });
});
