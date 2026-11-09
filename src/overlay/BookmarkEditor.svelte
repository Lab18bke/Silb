<script lang="ts">
  import { untrack } from 'svelte';
  import type { Bookmark } from '../lib/types';
  import type { FolderOption } from '../lib/bookmarks';

  let {
    data,
    onsave,
    onremove,
    oncancel,
  }: {
    data: { bookmark: Bookmark; folders: FolderOption[]; created: boolean; editUrl?: boolean; isPrivate: boolean };
    onsave: (d: { title: string; url: string | null; parentId: number }) => void;
    onremove: () => void;
    oncancel: () => void;
  } = $props();

  const b = untrack(() => data.bookmark);
  let title = $state(b.title);
  let url = $state(b.url ?? '');
  let parentId = $state(b.parentId ?? 1);

  function submit(e: SubmitEvent) {
    e.preventDefault();
    onsave({ title: title.trim(), url: b.isFolder ? null : url.trim() || b.url, parentId });
  }
</script>

<form class="editor" onsubmit={submit}>
  <h2>{b.isFolder ? 'Edit folder' : data.created ? 'Bookmark added' : 'Edit bookmark'}</h2>
  <label>
    <span>Name</span>
    <input class="field" bind:value={title} data-autofocus onfocus={(e) => (e.currentTarget as HTMLInputElement).select()} />
  </label>
  {#if !b.isFolder && data.editUrl}
    <label>
      <span>Address</span>
      <input class="field" bind:value={url} spellcheck="false" />
    </label>
  {/if}
  {#if !(b.isFolder && b.parentId === null)}
    <label>
      <span>Folder</span>
      <select class="field" bind:value={parentId}>
        {#each data.folders as f (f.id)}
          <option value={f.id}>{'   '.repeat(f.depth)}{f.title}</option>
        {/each}
      </select>
    </label>
  {/if}
  {#if data.isPrivate}
    <p class="note">Bookmarks are saved to your regular profile, even from a private window.</p>
  {/if}
  <div class="actions">
    <button type="button" class="btn quiet danger" onclick={onremove}>{data.created ? 'Remove' : 'Delete'}</button>
    <span class="spacer"></span>
    <button type="button" class="btn quiet" onclick={oncancel}>Cancel</button>
    <button type="submit" class="btn primary">Done</button>
  </div>
</form>

<style>
  .editor {
    padding: 16px 18px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h2 {
    margin: 0 0 4px;
    font-size: 15px;
    font-weight: 600;
  }
  label {
    display: grid;
    grid-template-columns: 64px 1fr;
    align-items: center;
    gap: 10px;
  }
  label span {
    color: var(--muted);
  }
  .note {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
  }
  .actions {
    display: flex;
    gap: 6px;
    margin-top: 6px;
  }
  .spacer {
    flex: 1;
  }
</style>
