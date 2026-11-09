<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Menu from './Menu.svelte';
  import Suggestions from './Suggestions.svelte';
  import BookmarkEditor from './BookmarkEditor.svelte';
  import PermissionPrompt from './PermissionPrompt.svelte';
  import DownloadsPanel from './DownloadsPanel.svelte';
  import SiteInfo from './SiteInfo.svelte';
  import Popups from './Popups.svelte';
  import Confirm from './Confirm.svelte';
  import { api } from '../lib/api';
  import { on } from '../lib/events';
  import { settings, applyTheme } from '../lib/settings.svelte';

  applyTheme();

  interface Payload {
    id: number;
    kind: string;
    data: any;
    margin: { top: number; side: number; bottom: number };
  }

  let current = $state<Payload | null>(null);
  let panel = $state<HTMLElement>();
  let toastTimer: number | undefined;

  function send(name: string, data: Record<string, unknown> = {}) {
    if (!current) return;
    api.overlayEvent(name, { id: current.id, ...data });
  }

  function close() {
    api.overlayHide();
  }

  function sendAndClose(name: string, data: Record<string, unknown> = {}) {
    send(name, data);
    close();
  }

  $effect(() => {
    if (!panel || !current) return;
    const m = current.margin;
    const kind = current.kind;
    if (kind === 'confirm') return;
    const ro = new ResizeObserver(() => {
      if (!panel) return;
      api.overlayResize(Math.ceil(panel.getBoundingClientRect().height) + m.top + m.bottom);
    });
    ro.observe(panel);
    return () => ro.disconnect();
  });

  onMount(() => {
    settings.load();
    const offs = [
      on<Payload>('overlay-show', async (p) => {
        clearTimeout(toastTimer);
        current = p;
        await tick();
        if (p.kind === 'toast') toastTimer = window.setTimeout(close, 2600);
        if (p.kind !== 'suggestions' && p.kind !== 'toast') {
          window.focus();
          panel?.querySelector<HTMLElement>('[data-autofocus]')?.focus();
        }
      }),
      on('overlay-hidden', () => (current = null)),
    ];
    const onBlur = () => {
      if (current && current.kind !== 'suggestions' && current.kind !== 'toast' && !(current.kind === 'downloads' && current.data?.auto)) close();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && current) {
        e.preventDefault();
        if (current.kind === 'confirm') send('cancel');
        close();
      }
    };
    window.addEventListener('blur', onBlur);
    window.addEventListener('keydown', onKey);
    return () => {
      offs.forEach((o) => o());
      window.removeEventListener('blur', onBlur);
      window.removeEventListener('keydown', onKey);
    };
  });
</script>

{#if current}
  {#key current.id}
    {#if current.kind === 'confirm'}
      <Confirm data={current.data} onconfirm={() => sendAndClose('confirm')} oncancel={() => sendAndClose('cancel')} />
    {:else}
      <div class="wrap" style:padding="{current.margin.top}px {current.margin.side}px {current.margin.bottom}px">
        <div class="panel" class:toast={current.kind === 'toast'} class:flat={current.kind === 'suggestions'} bind:this={panel} role="dialog">
          {#if current.kind === 'menu'}
            <Menu items={current.data.items} onselect={(itemId, newTab) => sendAndClose('select', { itemId, newTab })} onzoom={(action) => send('zoom', { action })} />
          {:else if current.kind === 'suggestions'}
            <Suggestions data={current.data} onpick={(index, newTab) => send('pick', { index, newTab })} onhover={(index) => send('hover', { index })} />
          {:else if current.kind === 'bookmark'}
            <BookmarkEditor data={current.data} onsave={(d) => sendAndClose('save', d)} onremove={() => sendAndClose('remove')} oncancel={close} />
          {:else if current.kind === 'permission'}
            <PermissionPrompt data={current.data} onresolve={(allow, remember) => sendAndClose('resolve', { allow, remember })} />
          {:else if current.kind === 'downloads'}
            <DownloadsPanel auto={!!current.data?.auto} onshowall={() => sendAndClose('show-all')} onclose={close} />
          {:else if current.kind === 'siteinfo'}
            <SiteInfo data={current.data} onclose={close} />
          {:else if current.kind === 'popups'}
            <Popups data={current.data} onclose={close} />
          {:else if current.kind === 'toast'}
            <div class="toast-text" role="status">{current.data.message}</div>
          {/if}
        </div>
      </div>
    {/if}
  {/key}
{/if}

<style>
  :global(body) {
    background: transparent;
  }
  .wrap {
    width: 100vw;
    min-height: 0;
  }
  .panel {
    background: var(--raised);
    color: var(--ink);
    border: 1px solid var(--line);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow-pop);
    overflow: hidden;
    animation: appear 0.12s ease-out;
  }
  .panel.flat {
    border-top-left-radius: var(--radius-m);
    border-top-right-radius: var(--radius-m);
    animation: none;
  }
  .panel.toast {
    border-radius: 20px;
    background: var(--ink);
    color: var(--surface);
    border-color: transparent;
  }
  .toast-text {
    padding: 10px 18px;
    text-align: center;
    font-size: 13px;
  }
  @keyframes appear {
    from {
      opacity: 0;
      transform: translateY(-3px);
    }
  }
</style>
