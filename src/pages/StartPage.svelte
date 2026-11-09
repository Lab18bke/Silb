<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../components/Icon.svelte';
  import Favicon from '../components/Favicon.svelte';
  import Brand from '../components/Brand.svelte';
  import { api, errorText } from '../lib/api';
  import { greeting, hostOf } from '../lib/format';
  import { settings } from '../lib/settings.svelte';
  import { browser } from '../chrome/state.svelte';
  import { bus, openInternal, openUrl } from '../chrome/actions';
  import { toast } from '../chrome/popover';
  import type { TopSite } from '../lib/types';

  let input: HTMLInputElement;
  let text = $state('');
  let sites = $state<TopSite[]>([]);
  const hello = greeting();

  const engine = $derived.by(() => {
    const s = settings.value;
    const custom = s?.customEngines.find((c) => c.id === s.searchEngine);
    return custom?.name ?? { google: 'Google', bing: 'Bing', duckduckgo: 'DuckDuckGo', brave: 'Brave Search' }[s?.searchEngine ?? 'google'] ?? 'Google';
  });

  async function submit(e: SubmitEvent) {
    e.preventDefault();
    if (!text.trim() || browser.active == null) return;
    try {
      await api.tabNavigate(browser.active, text);
    } catch (err) {
      toast(errorText(err));
    }
  }

  function open(url: string, e: MouseEvent) {
    e.preventDefault();
    openUrl(url, e.ctrlKey || e.button === 1 ? 'background' : 'current');
  }

  async function load() {
    if (settings.value?.showTopSites !== false) sites = await api.topSites(8);
  }

  onMount(() => {
    load();
    const focus = () => input?.focus();
    bus.addEventListener('focus-page', focus);
    bus.addEventListener('reload-page', load);
    requestAnimationFrame(focus);
    return () => {
      bus.removeEventListener('focus-page', focus);
      bus.removeEventListener('reload-page', load);
    };
  });
</script>

<div class="start">
  <div class="column">
    <h1>{hello}</h1>
    <form onsubmit={submit} class="search">
      <span class="mark"><Brand height={26} isDark={settings.dark} /></span>
      <input
        bind:this={input}
        bind:value={text}
        placeholder="Search {engine} or type an address"
        aria-label="Search or type an address"
        spellcheck="false"
        autocomplete="off"
      />
    </form>

    {#if settings.value?.showTopSites !== false && sites.length}
      <ul class="sites" aria-label="Frequently visited">
        {#each sites as site (site.url)}
          <li>
            <a href={site.url} onclick={(e) => open(site.url, e)} onauxclick={(e) => open(site.url, e)} title={site.url}>
              <span class="tile"><Favicon src={site.favicon} size={22} /></span>
              <span class="label">{site.title || hostOf(site.url)}</span>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <nav class="quick" aria-label="Shortcuts">
    <button type="button" onclick={() => openInternal('bookmarks')}><Icon name="bookmark" size={15} />Bookmarks</button>
    <button type="button" onclick={() => openInternal('history')}><Icon name="clock" size={15} />History</button>
    <button type="button" onclick={() => openInternal('downloads')}><Icon name="download" size={15} />Downloads</button>
  </nav>
</div>

<style>
  .start {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    background:
      radial-gradient(1200px 500px at 50% -10%, color-mix(in oklab, var(--accent) 5%, transparent), transparent 70%),
      var(--surface);
  }
  .column {
    width: min(640px, calc(100% - 48px));
    margin-top: max(9vh, 48px);
    display: flex;
    flex-direction: column;
    align-items: center;
  }
  h1 {
    font-family: var(--font-display);
    font-optical-sizing: auto;
    font-weight: 380;
    font-size: clamp(38px, 6vw, 58px);
    line-height: 1.05;
    letter-spacing: -0.022em;
    color: var(--ink);
    margin: 0 0 34px;
    text-align: center;
  }
  .search {
    width: 100%;
    height: 54px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 20px 0 14px;
    border-radius: var(--radius-l);
    background: var(--raised);
    border: 1px solid var(--line-strong);
    color: var(--muted);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.04);
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px var(--accent-soft);
  }
  .mark {
    display: flex;
    flex: none;
  }
  .mark:empty::before {
    content: '';
    width: 18px;
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    font-size: 16px;
    color: var(--ink);
  }
  .sites {
    list-style: none;
    padding: 0;
    margin: 44px 0 0;
    width: 100%;
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 6px;
  }
  .sites li {
    width: 104px;
  }
  .sites a {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 12px 6px 10px;
    border-radius: var(--radius-m);
    text-decoration: none;
    color: var(--ink-2);
  }
  .sites a:hover {
    background: var(--hover);
  }
  .tile {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    background: var(--sunken);
    border: 1px solid var(--line);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .label {
    max-width: 100%;
    font-size: 12px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .quick {
    margin-top: auto;
    padding: 40px 0 28px;
    display: flex;
    gap: 6px;
  }
  .quick button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 12px;
    border: 0;
    border-radius: 16px;
    background: transparent;
    color: var(--muted);
  }
  .quick button:hover {
    background: var(--hover);
    color: var(--ink);
  }
</style>
