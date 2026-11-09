export type Security = 'secure' | 'insecure' | 'cert-error' | 'internal' | 'file' | 'none';

export interface TabInfo {
  id: number;
  url: string;
  title: string;
  favicon: string | null;
  loading: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  zoom: number;
  internal: string | null;
  crashed: string | null;
  readerAvailable: boolean;
  readerActive: boolean;
  security: Security;
  audible: boolean;
  muted: boolean;
  blockedPopups: string[];
}

export interface WindowSnapshot {
  label: string;
  private: boolean;
  tabs: TabInfo[];
  active: number | null;
  maximized: boolean;
  fullscreen: boolean;
  closedCount: number;
}

export interface CustomEngine {
  id: string;
  name: string;
  url: string;
}

export interface Settings {
  startup: 'new-tab' | 'restore' | 'homepage';
  homepage: string;
  newTabPage: 'start' | 'blank' | 'homepage';
  websiteLanguage: string;
  searchEngine: string;
  customEngines: CustomEngine[];
  theme: 'system' | 'light' | 'dark';
  accent: 'juniper' | 'ink' | 'clay' | 'plum' | 'graphite';
  density: 'comfortable' | 'compact';
  showBookmarksBar: boolean;
  showTopSites: boolean;
  openLinksInNewTab: boolean;
  switchToNewTabs: boolean;
  confirmCloseMultiple: boolean;
  suspendBackgroundTabs: boolean;
  downloadDir: string;
  askDownloadLocation: boolean;
  showDownloadsPopup: boolean;
  trackingPrevention: 'off' | 'basic' | 'balanced' | 'strict';
  blockPopups: boolean;
  readerFontSize: number;
  readerWidth: number;
  readerTheme: 'auto' | 'light' | 'sepia' | 'dark';
  readerFont: 'serif' | 'sans';
}

export interface HistoryEntry {
  id: number;
  url: string;
  title: string;
  visitedAt: number;
  favicon: string | null;
}

export interface TopSite {
  url: string;
  title: string;
  favicon: string | null;
  visits: number;
}

export interface Bookmark {
  id: number;
  parentId: number | null;
  isFolder: boolean;
  title: string;
  url: string | null;
  position: number;
  createdAt: number;
  favicon: string | null;
}

export type DownloadState = 'in_progress' | 'interrupted' | 'completed' | 'cancelled';

export interface DownloadRecord {
  id: number;
  url: string;
  path: string;
  fileName: string;
  mime: string;
  totalBytes: number | null;
  receivedBytes: number;
  state: DownloadState;
  error: string | null;
  startedAt: number;
  finishedAt: number | null;
  exists: boolean;
  private: boolean;
  canResume: boolean;
}

export interface Suggestion {
  kind: 'search' | 'url' | 'history' | 'bookmark';
  title: string;
  url: string;
  favicon: string | null;
}

export interface SiteSetting {
  origin: string;
  kind: string;
  value: 'allow' | 'block';
}

export interface SiteData {
  domain: string;
  cookies: number;
}

export interface SiteInfo {
  origin: string | null;
  host: string;
  security: Security;
  permissions: [string, 'allow' | 'block' | null][];
}

export interface PermissionPrompt {
  id: number;
  tabId: number | null;
  origin: string;
  kind: string;
  userInitiated: boolean;
}

export interface AppInfo {
  version: string;
  tauri: string;
  webview2: string;
  arch: string;
  dataDir: string;
  downloadDir: string;
  engines: [string, string, string][];
}

export interface ClearKinds {
  history: boolean;
  cookies: boolean;
  cache: boolean;
  downloads: boolean;
}

export const BOOKMARKS_BAR = 1;
export const OTHER_BOOKMARKS = 2;
