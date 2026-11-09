<script lang="ts">
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import { errorText } from '../../lib/api';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../chrome/popover';
  import type { Settings } from '../../lib/types';

  const s = $derived(settings.value!);
  let homepage = $state(settings.value?.homepage ?? '');
  let homepageError = $state('');
  const startupLanguage = settings.value?.websiteLanguage ?? '';
  const engines = $derived([
    { id: 'google', name: 'Google' },
    { id: 'bing', name: 'Bing' },
    { id: 'duckduckgo', name: 'DuckDuckGo' },
    { id: 'brave', name: 'Brave Search' },
    ...s.customEngines.map((c) => ({ id: c.id, name: c.name })),
  ]);

  const languages: [string, string][] = [
    ['', 'Same as Windows'],
    ['en-US', 'English (United States)'],
    ['en-GB', 'English (United Kingdom)'],
    ['es', 'Español'],
    ['fr', 'Français'],
    ['de', 'Deutsch'],
    ['it', 'Italiano'],
    ['pt-BR', 'Português (Brasil)'],
    ['nl', 'Nederlands'],
    ['pl', 'Polski'],
    ['sv', 'Svenska'],
    ['tr', 'Türkçe'],
    ['ru', 'Русский'],
    ['uk', 'Українська'],
    ['ar', 'العربية'],
    ['hi', 'हिन्दी'],
    ['ja', '日本語'],
    ['ko', '한국어'],
    ['zh-CN', '中文（简体）'],
    ['zh-TW', '中文（繁體）'],
  ];

  async function set(patch: Partial<Settings>) {
    try {
      await settings.update(patch);
    } catch (e) {
      toast(errorText(e));
    }
  }

  async function saveHomepage() {
    homepageError = '';
    let value = homepage.trim();
    if (value && !/^https?:\/\//i.test(value)) value = `https://${value}`;
    try {
      await settings.update({ homepage: value });
      homepage = value;
    } catch (e) {
      homepageError = errorText(e);
    }
  }
</script>

<Group title="On startup">
  <Row label="When Silb starts" description="Restoring brings back the tabs from your last session. Tabs load when you open them.">
    <Segmented
      label="When Silb starts"
      value={s.startup}
      options={[
        { value: 'new-tab', label: 'New tab' },
        { value: 'restore', label: 'Restore tabs' },
        { value: 'homepage', label: 'Homepage' },
      ]}
      onchange={(v) => set({ startup: v })}
    />
  </Row>
  <Row label="Homepage" description="Opened at startup when you choose Homepage, and by Alt+Home.">
    <form
      class="inline"
      onsubmit={(e) => {
        e.preventDefault();
        saveHomepage();
      }}
    >
      <input class="field" bind:value={homepage} onblur={saveHomepage} placeholder="https://" aria-label="Homepage address" spellcheck="false" />
    </form>
  </Row>
  {#if homepageError}<p class="error">{homepageError}</p>{/if}
</Group>

<Group title="Search">
  <Row label="Search engine" description="Used for searches from the address bar and the start page. Manage engines under Search.">
    <select class="field" value={s.searchEngine} onchange={(e) => set({ searchEngine: (e.currentTarget as HTMLSelectElement).value })} aria-label="Search engine">
      {#each engines as e (e.id)}
        <option value={e.id}>{e.name}</option>
      {/each}
    </select>
  </Row>
</Group>

<Group title="New tabs">
  <Row label="New tabs open" description="Applies to Ctrl+T and the + button.">
    <Segmented
      label="New tabs open"
      value={s.newTabPage}
      options={[
        { value: 'start', label: 'Start page' },
        { value: 'blank', label: 'Blank page' },
        { value: 'homepage', label: 'Homepage' },
      ]}
      onchange={(v) => set({ newTabPage: v })}
    />
  </Row>
</Group>

<Group title="Language">
  <Row
    label="Preferred language for websites"
    description="Sent to websites so they can show content in your language, and used by built-in dialogs. Takes effect after you restart Silb. Silb’s own menus are in English."
  >
    <select class="field" value={s.websiteLanguage} onchange={(e) => set({ websiteLanguage: (e.currentTarget as HTMLSelectElement).value })} aria-label="Preferred language">
      {#each languages as [code, name]}
        <option value={code}>{name}</option>
      {/each}
    </select>
  </Row>
  {#if s.websiteLanguage !== startupLanguage}
    <p class="hint">Restart Silb to apply the new language.</p>
  {/if}
</Group>

<style>
  .inline .field {
    width: 280px;
  }
  .error {
    color: var(--danger);
    margin: -6px 0 12px;
    font-size: 12.5px;
  }
  .hint {
    color: var(--accent);
    margin: -6px 0 12px;
    font-size: 12.5px;
  }
</style>
