import '@testing-library/jest-dom';

const storage = new Map<string, string>();
const localStorageMock: Storage = {
  get length() { return storage.size; },
  clear: () => storage.clear(),
  getItem: key => storage.get(key) ?? null,
  key: index => [...storage.keys()][index] ?? null,
  removeItem: key => storage.delete(key),
  setItem: (key, value) => storage.set(key, String(value)),
};

Object.defineProperty(window, 'localStorage', { configurable: true, value: localStorageMock });

// Mock the Tauri API — it doesn't exist in jsdom
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}));
