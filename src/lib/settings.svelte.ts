import { api } from './api';
import { on, isPrivateWindow } from './events';
import type { Settings } from './types';

class SettingsStore {
  value = $state<Settings | null>(null);
  systemDark = $state(window.matchMedia('(prefers-color-scheme: dark)').matches);

  async load() {
    this.value = await api.settingsGet();
    on<Settings>('settings-changed', (s) => (this.value = s));
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => (this.systemDark = e.matches));
  }

  get dark() {
    const t = this.value?.theme ?? 'system';
    return isPrivateWindow || t === 'dark' || (t === 'system' && this.systemDark);
  }

  async update(patch: Partial<Settings>) {
    if (!this.value) return;
    const next = { ...this.value, ...patch };
    this.value = next;
    try {
      this.value = await api.settingsSet(next);
    } catch (e) {
      this.value = await api.settingsGet();
      throw e;
    }
  }
}

export const settings = new SettingsStore();

export function applyTheme() {
  $effect(() => {
    const root = document.documentElement;
    const s = settings.value;
    root.dataset.theme = settings.dark ? 'dark' : 'light';
    root.dataset.private = String(isPrivateWindow);
    root.dataset.accent = s?.accent ?? 'juniper';
    root.dataset.density = s?.density ?? 'comfortable';
  });
}
