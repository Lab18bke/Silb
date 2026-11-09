import { invoke } from '@tauri-apps/api/core';
import type {
  AppInfo,
  Bookmark,
  ClearKinds,
  DownloadRecord,
  HistoryEntry,
  PermissionPrompt,
  Settings,
  SiteData,
  SiteInfo,
  SiteSetting,
  Suggestion,
  TopSite,
  WindowSnapshot,
} from './types';

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export const api = {
  uiReady: () => invoke<WindowSnapshot>('ui_ready'),
  windowSnapshot: () => invoke<WindowSnapshot>('window_snapshot'),
  windowControl: (action: 'minimize' | 'toggle-maximize' | 'close' | 'close-force' | 'fullscreen') =>
    invoke<void>('window_control', { action }),
  windowNew: (isPrivate: boolean, url?: string) => invoke<void>('window_new', { private: isPrivate, url }),
  setChromeTop: (top: number) => invoke<void>('set_chrome_top', { top }),
  setContentVisible: (visible: boolean) => invoke<void>('set_content_visible', { visible }),
  focusContent: () => invoke<void>('focus_content'),

  tabNew: (url?: string, active = true, after?: number) => invoke<number | null>('tab_new', { url, active, after }),
  tabClose: (id: number) => invoke<void>('tab_close', { id }),
  tabCloseOthers: (id: number) => invoke<void>('tab_close_others', { id }),
  tabActivate: (id: number) => invoke<void>('tab_activate', { id }),
  tabMove: (id: number, index: number) => invoke<void>('tab_move', { id, index }),
  tabBack: (id: number) => invoke<void>('tab_back', { id }),
  tabForward: (id: number) => invoke<void>('tab_forward', { id }),
  tabReload: (id: number, hard = false) => invoke<void>('tab_reload', { id, hard }),
  tabStop: (id: number) => invoke<void>('tab_stop', { id }),
  tabMute: (id: number) => invoke<void>('tab_mute', { id }),
  tabReader: (id: number) => invoke<void>('tab_reader', { id }),
  tabDevtools: (id: number) => invoke<void>('tab_devtools', { id }),
  tabPrint: (id: number) => invoke<void>('tab_print', { id }),
  tabNavigate: (id: number, input: string) => invoke<void>('tab_navigate', { id, input }),
  tabZoom: (id: number, action: 'in' | 'out' | 'reset') => invoke<number>('tab_zoom', { id, action }),
  tabReopen: () => invoke<void>('tab_reopen'),
  tabDuplicate: (id: number) => invoke<void>('tab_duplicate', { id }),
  tabOpenPopup: (id: number, url: string | null, allowSite: boolean) =>
    invoke<void>('tab_open_popup', { id, url, allowSite }),

  findStart: (term: string, caseSensitive: boolean) => invoke<void>('find_start', { term, caseSensitive }),
  findStep: (forward: boolean) => invoke<void>('find_step', { forward }),
  findStop: () => invoke<void>('find_stop'),

  overlayShow: (rect: Rect, payload: unknown, focus: boolean) => invoke<void>('overlay_show', { rect, payload, focus }),
  overlayHide: () => invoke<void>('overlay_hide'),
  overlayEvent: (name: string, data: unknown) => invoke<void>('overlay_event', { name, data }),
  overlayResize: (height: number) => invoke<void>('overlay_resize', { height }),

  suggest: (query: string) => invoke<Suggestion[]>('suggest', { query }),
  historyQuery: (search: string, before?: number, limit = 100) =>
    invoke<HistoryEntry[]>('history_query', { search, before, limit }),
  historyDelete: (ids: number[]) => invoke<void>('history_delete', { ids }),
  topSites: (limit = 8) => invoke<TopSite[]>('top_sites', { limit }),

  bookmarksAll: () => invoke<Bookmark[]>('bookmarks_all'),
  bookmarkCreate: (parentId: number, title: string, url: string | null) =>
    invoke<Bookmark>('bookmark_create', { parentId, title, url }),
  bookmarkUpdate: (id: number, title: string, url: string | null) =>
    invoke<Bookmark>('bookmark_update', { id, title, url }),
  bookmarkMove: (id: number, parentId: number, index: number) =>
    invoke<void>('bookmark_move', { id, parentId, index }),
  bookmarkDelete: (id: number) => invoke<void>('bookmark_delete', { id }),
  bookmarksForUrl: (url: string) => invoke<Bookmark[]>('bookmarks_for_url', { url }),
  bookmarksSearch: (query: string) => invoke<Bookmark[]>('bookmarks_search', { query }),

  downloadsList: () => invoke<DownloadRecord[]>('downloads_list'),
  downloadCancel: (id: number) => invoke<void>('download_cancel', { id }),
  downloadRetry: (id: number) => invoke<void>('download_retry', { id }),
  downloadOpen: (id: number) => invoke<void>('download_open', { id }),
  downloadReveal: (id: number) => invoke<void>('download_reveal', { id }),
  downloadRemove: (id: number) => invoke<void>('download_remove', { id }),
  downloadsClear: () => invoke<void>('downloads_clear'),

  settingsGet: () => invoke<Settings>('settings_get'),
  settingsSet: (settings: Settings) => invoke<Settings>('settings_set', { settings }),
  pickDownloadDir: () => invoke<string | null>('pick_download_dir'),

  clearBrowsingData: (kinds: ClearKinds, since: number | null) => invoke<void>('clear_browsing_data', { kinds, since }),
  siteDataList: () => invoke<SiteData[]>('site_data_list'),
  siteDataClear: (domain: string) => invoke<void>('site_data_clear', { domain }),
  siteSettingsList: () => invoke<SiteSetting[]>('site_settings_list'),
  siteSettingSet: (origin: string, kind: string, value: 'allow' | 'block' | null) =>
    invoke<void>('site_setting_set', { origin, kind, value }),
  siteInfo: (id: number) => invoke<SiteInfo>('site_info', { id }),
  permissionResolve: (id: number, allow: boolean, remember: boolean) =>
    invoke<void>('permission_resolve', { id, allow, remember }),
  permissionsPending: () => invoke<PermissionPrompt[]>('permissions_pending'),
  appInfo: () => invoke<AppInfo>('app_info'),
};

export function errorText(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return 'Something went wrong.';
}
