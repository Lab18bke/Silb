<script lang="ts">
  import Icon from '../components/Icon.svelte';
  import General from './settings/General.svelte';
  import Appearance from './settings/Appearance.svelte';
  import Tabs from './settings/Tabs.svelte';
  import Downloads from './settings/Downloads.svelte';
  import Privacy from './settings/Privacy.svelte';
  import Reading from './settings/Reading.svelte';
  import Search from './settings/Search.svelte';
  import About from './settings/About.svelte';
  import { api } from '../lib/api';
  import { settings } from '../lib/settings.svelte';
  import { browser } from '../chrome/state.svelte';
  import type { IconName } from '../lib/icons';

  let { section, clearDialog }: { section: string; clearDialog: boolean } = $props();

  const sections: { id: string; label: string; icon: IconName }[] = [
    { id: 'general', label: 'General', icon: 'home' },
    { id: 'appearance', label: 'Appearance', icon: 'sun' },
    { id: 'tabs', label: 'Tabs', icon: 'window' },
    { id: 'downloads', label: 'Downloads', icon: 'download' },
    { id: 'privacy', label: 'Privacy & security', icon: 'shield' },
    { id: 'reading', label: 'Reading', icon: 'reader' },
    { id: 'search', label: 'Search', icon: 'search' },
    { id: 'about', label: 'About Silb', icon: 'info' },
  ];

  const current = $derived(sections.find((s) => s.id === section) ?? sections[0]);

  function go(id: string) {
    if (browser.active != null) api.tabNavigate(browser.active, `silb://settings/${id}`);
  }
</script>

<div class="settings">
  <nav aria-label="Settings sections">
    <h1>Settings</h1>
    {#each sections as s (s.id)}
      <button type="button" class:active={s.id === current.id} aria-current={s.id === current.id ? 'page' : undefined} onclick={() => go(s.id)}>
        <Icon name={s.icon} size={15} />
        {s.label}
      </button>
    {/each}
  </nav>
  <div class="content scroll">
    <div class="inner">
      <h2>{current.label}</h2>
      {#if settings.value}
        {#if current.id === 'general'}<General />
        {:else if current.id === 'appearance'}<Appearance />
        {:else if current.id === 'tabs'}<Tabs />
        {:else if current.id === 'downloads'}<Downloads />
        {:else if current.id === 'privacy'}<Privacy openClear={clearDialog} />
        {:else if current.id === 'reading'}<Reading />
        {:else if current.id === 'search'}<Search />
        {:else}<About />
        {/if}
      {/if}
    </div>
  </div>
</div>

<style>
  .settings {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-columns: 248px 1fr;
    background: var(--surface);
  }
  nav {
    padding: 36px 14px 24px 20px;
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
  }
  h1 {
    font-family: var(--font-display);
    font-weight: 420;
    font-size: 30px;
    letter-spacing: -0.015em;
    margin: 0 0 18px 10px;
  }
  nav button {
    height: 34px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 10px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--ink-2);
    text-align: left;
  }
  nav button:hover {
    background: var(--hover);
  }
  nav button.active {
    background: var(--accent-soft);
    color: var(--ink);
    font-weight: 600;
  }
  .content {
    min-width: 0;
  }
  .inner {
    max-width: 720px;
    padding: 40px 40px 72px;
  }
  h2 {
    font-family: var(--font-display);
    font-weight: 450;
    font-size: 24px;
    margin: 4px 0 22px;
  }
  @media (max-width: 760px) {
    .settings {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
    }
    nav {
      flex-direction: row;
      flex-wrap: wrap;
      border-right: 0;
      border-bottom: 1px solid var(--line);
      padding: 14px;
    }
    h1 {
      display: none;
    }
  }
</style>
