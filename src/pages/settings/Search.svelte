<script lang="ts">
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { errorText } from '../../lib/api';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../chrome/popover';

  const s = $derived(settings.value!);
  const builtins = [
    { id: 'google', name: 'Google', url: 'https://www.google.com/search?q=%s' },
    { id: 'bing', name: 'Bing', url: 'https://www.bing.com/search?q=%s' },
    { id: 'duckduckgo', name: 'DuckDuckGo', url: 'https://duckduckgo.com/?q=%s' },
    { id: 'brave', name: 'Brave Search', url: 'https://search.brave.com/search?q=%s' },
  ];
  const engines = $derived([...builtins, ...s.customEngines]);

  let name = $state('');
  let url = $state('');
  let error = $state('');

  async function add(e: SubmitEvent) {
    e.preventDefault();
    error = '';
    const n = name.trim();
    const u = url.trim();
    if (!n || !/^https?:\/\//i.test(u) || !u.includes('%s')) {
      error = 'Enter a name and an http(s) address with %s where the search terms go.';
      return;
    }
    const id = `custom-${Date.now()}`;
    try {
      await settings.update({ customEngines: [...s.customEngines, { id, name: n, url: u }], searchEngine: id });
      name = '';
      url = '';
    } catch (err) {
      error = errorText(err);
    }
  }

  async function remove(id: string) {
    try {
      await settings.update({
        customEngines: s.customEngines.filter((c) => c.id !== id),
        searchEngine: s.searchEngine === id ? 'google' : s.searchEngine,
      });
    } catch (err) {
      toast(errorText(err));
    }
  }
</script>

<Group title="Default search engine">
  <Row label="Search with" description="Used when you type something that isn’t an address in the address bar or on the start page.">
    <select class="field" value={s.searchEngine} onchange={(e) => settings.update({ searchEngine: (e.currentTarget as HTMLSelectElement).value })} aria-label="Default search engine">
      {#each engines as engine (engine.id)}
        <option value={engine.id}>{engine.name}</option>
      {/each}
    </select>
  </Row>
</Group>

<Group title="Search engines">
  {#each engines as engine (engine.id)}
    <Row label={engine.name} description={engine.url}>
      {#if engine.id === s.searchEngine}
        <span class="default">Default</span>
      {:else}
        <button class="btn quiet" onclick={() => settings.update({ searchEngine: engine.id })}>Make default</button>
      {/if}
      {#if engine.id.startsWith('custom-')}
        <IconButton icon="trash" label="Remove {engine.name}" onclick={() => remove(engine.id)} />
      {/if}
    </Row>
  {/each}
</Group>

<Group title="Add a search engine">
  <form class="add" onsubmit={add}>
    <input class="field" bind:value={name} placeholder="Name" aria-label="Search engine name" />
    <input class="field url" bind:value={url} placeholder="https://example.com/search?q=%s" aria-label="Search URL" spellcheck="false" />
    <button class="btn primary" type="submit">Add</button>
  </form>
  {#if error}<p class="error">{error}</p>{/if}
  <p class="hint">Use %s where the search terms should go.</p>
</Group>

<style>
  .default {
    color: var(--accent);
    font-weight: 600;
    font-size: 12px;
    padding: 0 8px;
  }
  .add {
    display: flex;
    gap: 8px;
    padding: 14px 0 6px;
  }
  .add .field {
    flex: 1;
  }
  .add .url {
    flex: 2;
  }
  .error {
    color: var(--danger);
    margin: 4px 0;
    font-size: 12.5px;
  }
  .hint {
    color: var(--muted);
    font-size: 12px;
    margin: 4px 0 12px;
  }
</style>
