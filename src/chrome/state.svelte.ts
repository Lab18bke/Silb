import { api } from '../lib/api';
import { on, isPrivateWindow } from '../lib/events';
import type { PermissionPrompt, TabInfo, WindowSnapshot } from '../lib/types';

class BrowserState {
  tabs = $state<TabInfo[]>([]);
  active = $state<number | null>(null);
  maximized = $state(false);
  fullscreen = $state(false);
  htmlFullscreen = $state(false);
  closedCount = $state(0);
  findOpen = $state(false);
  prompts = $state<PermissionPrompt[]>([]);
  readonly isPrivate = isPrivateWindow;

  activeTab = $derived(this.tabs.find((t) => t.id === this.active) ?? null);

  apply(s: WindowSnapshot) {
    this.tabs = s.tabs;
    this.active = s.active;
    this.maximized = s.maximized;
    this.fullscreen = s.fullscreen;
    this.closedCount = s.closedCount;
  }

  async init() {
    on<WindowSnapshot>('tabs-changed', (s) => this.apply(s));
    on<TabInfo>('tab-updated', (t) => {
      const i = this.tabs.findIndex((x) => x.id === t.id);
      if (i >= 0) this.tabs[i] = t;
    });
    on<{ maximized: boolean; fullscreen: boolean }>('window-state', (s) => {
      this.maximized = s.maximized;
      this.fullscreen = s.fullscreen;
    });
    on<boolean>('html-fullscreen', (v) => (this.htmlFullscreen = v));
    on<PermissionPrompt>('permission-request', (p) => (this.prompts = [...this.prompts, p]));
    on<number>('permission-cancel', (id) => (this.prompts = this.prompts.filter((p) => p.id !== id)));
    this.apply(await api.uiReady());
    this.prompts = await api.permissionsPending();
  }
}

export const browser = new BrowserState();

export function tabDisplayUrl(t: TabInfo | null): string {
  if (!t) return '';
  if (t.internal) return t.internal === 'newtab' || t.internal === 'private' ? '' : `silb://${t.internal}`;
  return t.url === 'about:blank' ? '' : t.url;
}
