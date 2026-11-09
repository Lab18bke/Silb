<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from './Icon.svelte';
  import { api, errorText } from '../lib/api';
  import { browser, tabDisplayUrl } from '../chrome/state.svelte';
  import { bus } from '../chrome/actions';
  import { anchorBelow, closePopover, openPopover, toast, updatePopover } from '../chrome/popover';
  import { settings } from '../lib/settings.svelte';
  import type { Suggestion, TabInfo } from '../lib/types';
  import type { IconName } from '../lib/icons';
  import type { Snippet } from 'svelte';

  let { trailing }: { trailing?: Snippet } = $props();

  let input: HTMLInputElement;
  let box: HTMLDivElement;
  let chip = $state<HTMLButtonElement>();
  let focused = $state(false);
  let text = $state('');
  let typed = $state('');
  let items = $state<Suggestion[]>([]);
  let selected = $state(-1);
  let open = false;
  let seq = 0;
  let blurTimer: number | undefined;

  const tab = $derived(browser.activeTab);
  const display = $derived(tabDisplayUrl(tab));

  $effect(() => {
    display;
    tab?.id;
    if (!focused) text = display;
  });

  const engineName = $derived.by(() => {
    const s = settings.value;
    if (!s) return 'Google';
    const custom = s.customEngines.find((c) => c.id === s.searchEngine);
    if (custom) return custom.name;
    return { google: 'Google', bing: 'Bing', duckduckgo: 'DuckDuckGo', brave: 'Brave Search' }[s.searchEngine] ?? 'Google';
  });

  const security = $derived.by((): { icon: IconName; label: string; text?: string; tone?: string } => {
    if (focused) return { icon: 'search', label: 'Search or type an address' };
    if (!tab || tab.internal) return { icon: 'search', label: 'Silb page' };
    switch (tab.security) {
      case 'secure':
        return { icon: 'lock', label: 'Connection is secure' };
      case 'insecure':
        return { icon: 'warning', label: 'Connection is not secure', text: 'Not secure', tone: 'warn' };
      case 'cert-error':
        return { icon: 'warning', label: 'Certificate problem', text: 'Not secure', tone: 'danger' };
      case 'file':
        return { icon: 'page', label: 'Local file' };
      default:
        return { icon: 'globe', label: 'Page info' };
    }
  });

  function panelRect(count: number) {
    return anchorBelow(box, box.offsetWidth, Math.min(count, 8) * 38 + 12, 'start', 5);
  }

  function showSuggestions() {
    if (!focused || items.length === 0) {
      hideSuggestions();
      return;
    }
    const data = { items, selected };
    if (open) updatePopover('suggestions', data, panelRect(items.length));
    else {
      open = true;
      openPopover('suggestions', panelRect(items.length), data, {
        focus: false,
        margin: { top: 0 },
        onEvent: (name, d) => {
          if (name === 'pick') go(items[d.index]?.url ?? text, d.newTab);
          if (name === 'hover') selected = d.index;
        },
        onClose: () => (open = false),
      });
    }
  }

  function hideSuggestions() {
    if (open) closePopover('suggestions');
    open = false;
  }

  async function query(q: string) {
    const n = ++seq;
    if (!q.trim()) {
      items = [];
      hideSuggestions();
      return;
    }
    try {
      const result = await api.suggest(q);
      if (n !== seq || !focused) return;
      items = result;
      selected = -1;
      showSuggestions();
    } catch {
      items = [];
    }
  }

  function onInput() {
    typed = text;
    query(text);
  }

  async function go(target: string, newTab = false) {
    const value = target.trim();
    hideSuggestions();
    if (!value || !tab) return;
    input.blur();
    try {
      if (newTab) await api.tabNew(value);
      else await api.tabNavigate(tab.id, value);
    } catch (e) {
      toast(errorText(e));
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      if (!items.length) return;
      e.preventDefault();
      const delta = e.key === 'ArrowDown' ? 1 : -1;
      selected = Math.max(-1, Math.min(items.length - 1, selected + delta));
      text = selected >= 0 ? items[selected].url : typed;
      showSuggestions();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      go(selected >= 0 ? items[selected].url : text, e.altKey);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      if (open) {
        hideSuggestions();
        text = typed;
      } else {
        text = display;
        input.blur();
        api.focusContent();
      }
    }
  }

  function onFocus() {
    clearTimeout(blurTimer);
    focused = true;
    typed = text;
    requestAnimationFrame(() => input.select());
  }

  function onBlur() {
    focused = false;
    blurTimer = window.setTimeout(() => {
      hideSuggestions();
      if (!focused) text = display;
    }, 180);
  }

  function siteInfo() {
    if (!tab || tab.internal || !chip) return;
    const t: TabInfo = tab;
    api.siteInfo(t.id).then((info) => {
      openPopover('siteinfo', anchorBelow(chip!, 340, 360), { tabId: t.id, info });
    });
  }

  onMount(() => {
    const focusIt = () => {
      input.focus();
      input.select();
    };
    bus.addEventListener('focus-address', focusIt);
    return () => bus.removeEventListener('focus-address', focusIt);
  });
</script>

<div class="bar" class:focused bind:this={box}>
  <button
    bind:this={chip}
    class="chip {security.tone ?? ''}"
    type="button"
    aria-label={security.label}
    title={security.label}
    onclick={siteInfo}
    tabindex="-1"
  >
    <Icon name={security.icon} size={15} />
    {#if security.text}<span>{security.text}</span>{/if}
  </button>
  <input
    bind:this={input}
    bind:value={text}
    oninput={onInput}
    onkeydown={onKey}
    onfocus={onFocus}
    onblur={onBlur}
    spellcheck="false"
    autocomplete="off"
    aria-label="Address and search bar"
    placeholder={browser.isPrivate ? `Search ${engineName} privately or type an address` : `Search ${engineName} or type an address`}
  />
  {@render trailing?.()}
</div>

<style>
  .bar {
    flex: 1;
    min-width: 120px;
    height: var(--control-h);
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 0 4px;
    border-radius: var(--radius-m);
    background: var(--sunken);
    border: 1px solid transparent;
    transition: background 0.12s, border-color 0.12s, box-shadow 0.12s;
  }
  .bar:hover {
    background: color-mix(in oklab, var(--sunken) 80%, var(--ink) 4%);
  }
  .bar.focused {
    background: var(--raised);
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .chip {
    height: calc(var(--control-h) - 8px);
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 0 7px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    white-space: nowrap;
  }
  .chip:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .chip.warn {
    color: var(--warn);
  }
  .chip.danger {
    color: var(--danger);
    font-weight: 600;
  }
  input {
    flex: 1;
    min-width: 0;
    height: 100%;
    border: 0;
    outline: none;
    background: transparent;
    padding: 0 4px;
    font-size: 13.5px;
    color: var(--ink);
  }
</style>
