<script lang="ts">
  import { onMount } from 'svelte';
  import TabStrip from '../components/TabStrip.svelte';
  import Toolbar from '../components/Toolbar.svelte';
  import BookmarksBar from '../components/BookmarksBar.svelte';
  import FindBar from '../components/FindBar.svelte';
  import Icon from '../components/Icon.svelte';
  import StartPage from '../pages/StartPage.svelte';
  import PrivatePage from '../pages/PrivatePage.svelte';
  import HistoryPage from '../pages/HistoryPage.svelte';
  import BookmarksPage from '../pages/BookmarksPage.svelte';
  import DownloadsPage from '../pages/DownloadsPage.svelte';
  import SettingsPage from '../pages/SettingsPage.svelte';
  import { api } from '../lib/api';
  import { on } from '../lib/events';
  import { settings, applyTheme } from '../lib/settings.svelte';
  import { browser } from './state.svelte';
  import { emitBus, runAction } from './actions';
  import { anchorBelow, initPopovers, onPopoverChange, openPopover, popoverKind, toast } from './popover';

  applyTheme();

  let header = $state<HTMLElement>();
  let ready = $state(false);
  let popover = $state<string | null>(null);
  let showingPrompt: number | null = null;

  const tab = $derived(browser.activeTab);
  const page = $derived(tab?.internal ?? null);
  const base = $derived(page?.split('/')[0] ?? null);
  const chromeHidden = $derived(browser.fullscreen && !browser.htmlFullscreen);

  $effect(() => {
    if (!header) return;
    let last = -1;
    const report = () => {
      const h = Math.round(header!.getBoundingClientRect().height);
      if (h !== last) {
        last = h;
        api.setChromeTop(h);
      }
    };
    const ro = new ResizeObserver(report);
    ro.observe(header);
    report();
    return () => ro.disconnect();
  });

  $effect(() => {
    const t = tab;
    const prompt = browser.prompts.find((p) => p.tabId === null || p.tabId === t?.id);
    if (!prompt || popover || showingPrompt === prompt.id) return;
    const chip = document.querySelector('.bar .chip');
    if (!chip) return;
    showingPrompt = prompt.id;
    let resolved = false;
    openPopover('permission', anchorBelow(chip, 360, 176), prompt, {
      onEvent: (name, data) => {
        if (name !== 'resolve') return;
        resolved = true;
        api.permissionResolve(prompt.id, data.allow, data.remember).catch(() => {});
      },
      onClose: () => {
        showingPrompt = null;
        if (!resolved) api.permissionResolve(prompt.id, false, false).catch(() => {});
      },
    });
  });

  function confirmClose(count: number) {
    openPopover(
      'confirm',
      { x: 0, y: 0, width: window.innerWidth, height: window.innerHeight },
      {
        title: `Close ${count} tabs?`,
        message: browser.isPrivate
          ? 'Closing this private window ends its tabs. If no other private windows are open, their cookies and site data are deleted.'
          : 'This window has more than one tab open.',
        confirm: 'Close window',
        cancel: 'Cancel',
      },
      {
        margin: { top: 0, side: 0, bottom: 0 },
        onEvent: (name) => {
          if (name === 'confirm') api.windowControl('close-force');
        },
      },
    );
  }

  onMount(() => {
    initPopovers();
    const offs = [
      onPopoverChange(() => (popover = popoverKind())),
      on<string>('shortcut', runAction),
      on('focus-page', () => emitBus('focus-page')),
      on<number>('reload-page', (id) => id === browser.active && emitBus('reload-page')),
      on<number>('confirm-close', confirmClose),
      on<string>('toast', (m) => toast(m)),
    ];
    Promise.all([settings.load(), browser.init()]).then(() => (ready = true));
    return () => offs.forEach((o) => o());
  });
</script>

<div class="app">
  <header bind:this={header} class:hidden={chromeHidden}>
    {#if !chromeHidden}
      <TabStrip />
      <Toolbar />
      {#if settings.value?.showBookmarksBar}
        <BookmarksBar />
      {/if}
    {/if}
    {#if browser.findOpen && tab && !tab.internal}
      <FindBar />
    {/if}
  </header>

  <main>
    {#if ready && tab}
      {#key tab.id}
        {#if base === 'newtab'}
          <StartPage />
        {:else if base === 'private'}
          <PrivatePage />
        {:else if base === 'history'}
          <HistoryPage />
        {:else if base === 'bookmarks'}
          <BookmarksPage />
        {:else if base === 'downloads'}
          <DownloadsPage />
        {:else if base === 'settings'}
          <SettingsPage section={page?.split('/')[1] ?? 'general'} clearDialog={page?.split('/')[2] === 'clear'} />
        {:else if tab.crashed}
          <div class="crash">
            <Icon name="warning" size={28} stroke={1.2} />
            <h1>{tab.crashed}</h1>
            <p>
              {tab.crashed.includes('responding')
                ? 'You can wait for it to respond or reload it.'
                : 'Reloading usually fixes this. If it keeps happening, the page may be using more memory than is available.'}
            </p>
            <button class="btn primary" onclick={() => api.tabReload(tab.id)}>Reload page</button>
          </div>
        {:else}
          <div class="blank" class:loading={tab.loading}></div>
        {/if}
      {/key}
    {/if}
  </main>
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  header {
    flex: none;
    position: relative;
    z-index: 1;
  }
  main {
    flex: 1;
    min-height: 0;
    background: var(--surface);
    position: relative;
  }
  .blank {
    position: absolute;
    inset: 0;
  }
  .blank.loading::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    height: 2px;
    width: 30%;
    background: var(--accent);
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(400%);
    }
  }
  .crash {
    max-width: 520px;
    margin: 14vh auto 0;
    padding: 0 24px;
    color: var(--muted);
  }
  .crash h1 {
    font-family: var(--font-display);
    font-weight: 500;
    font-size: 28px;
    color: var(--ink);
    margin: 18px 0 8px;
  }
  .crash p {
    margin: 0 0 22px;
    font-size: 14px;
    line-height: 1.6;
  }
</style>
