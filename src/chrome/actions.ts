import { api } from '../lib/api';
import { settings } from '../lib/settings.svelte';
import { browser } from './state.svelte';
import { toast } from './popover';

export const bus = new EventTarget();

export function emitBus(name: string, detail?: unknown) {
  bus.dispatchEvent(new CustomEvent(name, { detail }));
}

function activeId(): number | null {
  return browser.active;
}

export async function openInternal(page: string) {
  const base = page.split('/')[0];
  const existing = browser.tabs.find((t) => t.internal?.split('/')[0] === base);
  if (existing) {
    await api.tabActivate(existing.id);
    if (existing.internal !== page) await api.tabNavigate(existing.id, `silb://${page}`);
    return;
  }
  const tab = browser.activeTab;
  if (tab && (tab.internal === 'newtab' || tab.internal === 'private')) {
    await api.tabNavigate(tab.id, `silb://${page}`);
  } else {
    await api.tabNew(`silb://${page}`, true, tab?.id);
  }
}

export async function openUrl(url: string, mode: 'current' | 'background' | 'foreground' | 'window' = 'current') {
  const id = activeId();
  if (mode === 'window') return api.windowNew(browser.isPrivate, url);
  if (mode === 'current' && id != null) return api.tabNavigate(id, url);
  return api.tabNew(url, mode !== 'background', id ?? undefined);
}

function cycle(delta: number) {
  const i = browser.tabs.findIndex((t) => t.id === browser.active);
  if (i < 0 || browser.tabs.length < 2) return;
  const next = browser.tabs[(i + delta + browser.tabs.length) % browser.tabs.length];
  api.tabActivate(next.id);
}

function selectIndex(i: number) {
  const t = i < 0 ? browser.tabs[browser.tabs.length - 1] : browser.tabs[i];
  if (t) api.tabActivate(t.id);
}

const withActive = (fn: (id: number) => unknown) => () => {
  const id = activeId();
  if (id != null) return fn(id);
};

const webOnly = (fn: (id: number) => unknown) => () => {
  const t = browser.activeTab;
  if (t && !t.internal) return fn(t.id);
};

const handlers: Record<string, () => unknown> = {
  'new-tab': () => api.tabNew(),
  'close-tab': withActive(api.tabClose),
  'reopen-tab': () => api.tabReopen(),
  'next-tab': () => cycle(1),
  'prev-tab': () => cycle(-1),
  'tab-last': () => selectIndex(-1),
  'focus-address': () => emitBus('focus-address'),
  reload: withActive((id) => api.tabReload(id, false)),
  'hard-reload': withActive((id) => api.tabReload(id, true)),
  stop: withActive(api.tabStop),
  back: withActive(api.tabBack),
  forward: withActive(api.tabForward),
  home: withActive((id) => api.tabNavigate(id, settings.value?.homepage || 'silb://newtab')),
  bookmark: () => emitBus('bookmark-page'),
  'toggle-bookmarks-bar': () => settings.update({ showBookmarksBar: !settings.value?.showBookmarksBar }),
  bookmarks: () => openInternal('bookmarks'),
  history: () => openInternal('history'),
  downloads: () => openInternal('downloads'),
  settings: () => openInternal('settings'),
  'clear-data': () => openInternal('settings/privacy/clear'),
  'new-window': () => api.windowNew(false),
  'new-private-window': () => api.windowNew(true),
  'zoom-in': webOnly((id) => api.tabZoom(id, 'in')),
  'zoom-out': webOnly((id) => api.tabZoom(id, 'out')),
  'zoom-reset': webOnly((id) => api.tabZoom(id, 'reset')),
  fullscreen: async () => {
    await api.windowControl('fullscreen');
    const snap = await api.windowSnapshot();
    if (snap.fullscreen) toast('Press F11 to exit full screen');
  },
  find: webOnly(() => {
    browser.findOpen = true;
    emitBus('focus-find');
  }),
  'find-next': webOnly(() => (browser.findOpen ? api.findStep(true) : ((browser.findOpen = true), emitBus('focus-find')))),
  'find-prev': webOnly(() => browser.findOpen && api.findStep(false)),
  'view-source': webOnly((id) => {
    const t = browser.tabs.find((x) => x.id === id);
    if (t?.url.startsWith('http')) api.tabNew(`view-source:${t.url}`, true, id);
  }),
  print: webOnly(api.tabPrint),
  devtools: webOnly(api.tabDevtools),
  reader: webOnly(api.tabReader),
};

for (let i = 1; i <= 8; i++) handlers[`tab-${i}`] = () => selectIndex(i - 1);

export function runAction(name: string) {
  const fn = handlers[name];
  if (!fn) return;
  Promise.resolve(fn()).catch((e) => toast(typeof e === 'string' ? e : 'That didn’t work.'));
}
