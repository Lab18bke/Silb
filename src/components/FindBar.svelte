<script lang="ts">
  import { onMount } from 'svelte';
  import IconButton from './IconButton.svelte';
  import { api } from '../lib/api';
  import { on } from '../lib/events';
  import { bus } from '../chrome/actions';
  import { browser } from '../chrome/state.svelte';

  let input: HTMLInputElement;
  let term = $state('');
  let caseSensitive = $state(false);
  let result = $state<{ active: number; count: number } | null>(null);
  let timer: number | undefined;

  function search() {
    clearTimeout(timer);
    timer = window.setTimeout(() => {
      api.findStart(term, caseSensitive).catch(() => {});
      if (!term) result = null;
    }, 90);
  }

  function close() {
    browser.findOpen = false;
    api.findStop();
    api.focusContent();
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      api.findStep(!e.shiftKey);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      close();
    }
  }

  function toggleCase() {
    caseSensitive = !caseSensitive;
    search();
  }

  $effect(() => {
    browser.active;
    result = null;
  });

  onMount(() => {
    const focus = () => {
      input?.focus();
      input?.select();
      if (term) search();
    };
    bus.addEventListener('focus-find', focus);
    focus();
    const off = on<{ tabId: number; active: number; count: number }>('find-result', (r) => {
      if (r.tabId === browser.active && term) result = { active: r.active, count: r.count };
    });
    return () => {
      bus.removeEventListener('focus-find', focus);
      off();
    };
  });
</script>

<div class="find" role="search">
  <input
    bind:this={input}
    bind:value={term}
    oninput={search}
    onkeydown={onKey}
    class="field"
    placeholder="Find on page"
    aria-label="Find on page"
    spellcheck="false"
  />
  <span class="count" aria-live="polite">
    {#if result && term}
      {result.count === 0 ? 'No matches' : `${result.active} of ${result.count}`}
    {/if}
  </span>
  <IconButton icon="chevron-up" label="Previous match (Shift+Enter)" disabled={!result?.count} onclick={() => api.findStep(false)} />
  <IconButton icon="chevron-down" label="Next match (Enter)" disabled={!result?.count} onclick={() => api.findStep(true)} />
  <button type="button" class="case" aria-pressed={caseSensitive} title="Match case" onclick={toggleCase}>Aa</button>
  <IconButton icon="close" label="Close find bar (Esc)" onclick={close} />
</div>

<style>
  .find {
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 4px;
    padding: 0 10px;
    background: var(--surface);
    border-bottom: 1px solid var(--line);
  }
  .field {
    width: 260px;
  }
  .count {
    min-width: 84px;
    text-align: right;
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
    padding-right: 6px;
  }
  .case {
    height: var(--control-h);
    min-width: var(--control-h);
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--muted);
    font-weight: 600;
    font-size: 12px;
  }
  .case:hover {
    background: var(--hover);
  }
  .case[aria-pressed='true'] {
    color: var(--accent);
    background: var(--accent-soft);
  }
</style>
