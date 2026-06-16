import '@testing-library/jest-dom';

// Mock the Tauri API — it doesn't exist in jsdom
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}));
