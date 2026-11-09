<script lang="ts">
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import { api, errorText } from '../lib/api';
  import { formatBytes, hostOf } from '../lib/format';
  import type { DownloadRecord } from '../lib/types';

  let { d, compact = false, onerror }: { d: DownloadRecord; compact?: boolean; onerror?: (msg: string) => void } = $props();

  const pct = $derived(d.totalBytes ? Math.min(100, (d.receivedBytes / d.totalBytes) * 100) : null);
  const ext = $derived((d.fileName.match(/\.([a-z0-9]{1,5})$/i)?.[1] ?? '').toUpperCase());

  const status = $derived.by(() => {
    switch (d.state) {
      case 'in_progress':
        return d.totalBytes ? `${formatBytes(d.receivedBytes)} of ${formatBytes(d.totalBytes)}` : `${formatBytes(d.receivedBytes)} downloaded`;
      case 'completed':
        return d.exists ? [formatBytes(d.totalBytes ?? d.receivedBytes), hostOf(d.url)].filter(Boolean).join(', ') : 'Deleted or moved';
      case 'cancelled':
        return 'Cancelled';
      default:
        return d.error || 'Failed';
    }
  });

  async function act(fn: () => Promise<void>) {
    try {
      await fn();
    } catch (e) {
      onerror?.(errorText(e));
    }
  }

  const openable = $derived(d.state === 'completed' && d.exists);
</script>

<div class="row" class:compact class:faded={d.state === 'completed' && !d.exists}>
  <span class="icon" class:done={d.state === 'completed'} class:bad={d.state === 'interrupted'}>
    {#if ext && ext.length <= 4}<span class="ext">{ext}</span>{:else}<Icon name="page" size={16} />{/if}
  </span>
  <div class="body">
    {#if openable}
      <button type="button" class="name link" title="Open {d.fileName}" onclick={() => act(() => api.downloadOpen(d.id))}>{d.fileName}</button>
    {:else}
      <span class="name" title={d.fileName}>{d.fileName}</span>
    {/if}
    <span class="status" class:err={d.state === 'interrupted'}>{status}</span>
    {#if d.state === 'in_progress'}
      <span class="bar" class:indeterminate={pct === null}><span style:width={pct === null ? '30%' : `${pct}%`}></span></span>
    {/if}
    {#if !compact}
      <span class="src" title={d.url}>{d.url.startsWith('http') ? d.url : ''}</span>
    {/if}
  </div>
  <div class="acts">
    {#if d.state === 'in_progress'}
      <IconButton icon="close" label="Cancel download" onclick={() => act(() => api.downloadCancel(d.id))} />
    {:else}
      {#if openable}
        <IconButton icon="folder" label="Show in folder" onclick={() => act(() => api.downloadReveal(d.id))} />
      {/if}
      {#if d.state === 'interrupted' || d.state === 'cancelled' || (d.state === 'completed' && !d.exists)}
        <IconButton icon="retry" label={d.canResume ? 'Resume download' : 'Retry download'} onclick={() => act(() => api.downloadRetry(d.id))} />
      {/if}
      {#if !compact}
        <IconButton icon="close-small" label="Remove from list (the file is kept)" onclick={() => act(() => api.downloadRemove(d.id))} />
      {/if}
    {/if}
  </div>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 8px 12px 12px;
    border-radius: var(--radius-m);
  }
  .row.compact {
    gap: 12px;
    padding: 8px 6px 8px 10px;
  }
  .row:hover {
    background: var(--hover);
  }
  .faded .name {
    color: var(--muted);
    text-decoration: line-through;
  }
  .icon {
    flex: none;
    width: 36px;
    height: 40px;
    border-radius: 7px;
    border: 1px solid var(--line-strong);
    background: var(--raised);
    color: var(--muted);
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .compact .icon {
    width: 30px;
    height: 34px;
  }
  .icon.done {
    border-color: color-mix(in oklab, var(--accent) 45%, var(--line-strong));
    color: var(--accent);
  }
  .icon.bad {
    color: var(--danger);
  }
  .ext {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.02em;
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .name {
    align-self: flex-start;
    max-width: 100%;
    font-weight: 600;
    color: var(--ink);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name.link {
    border: 0;
    padding: 0;
    background: none;
    text-align: left;
  }
  .name.link:hover {
    color: var(--accent);
    text-decoration: underline;
  }
  .status {
    color: var(--muted);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .status.err {
    color: var(--danger);
  }
  .src {
    color: var(--faint);
    font-size: 11.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .bar {
    margin-top: 5px;
    height: 4px;
    border-radius: 2px;
    background: var(--line);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    background: var(--accent);
    border-radius: 2px;
    transition: width 0.25s;
  }
  .bar.indeterminate span {
    animation: slide 1.3s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
  .acts {
    display: flex;
    flex: none;
  }
</style>
