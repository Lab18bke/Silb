<script lang="ts">
  import { onMount } from 'svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Brand from '../../components/Brand.svelte';
  import { api } from '../../lib/api';
  import { settings } from '../../lib/settings.svelte';
  import type { AppInfo } from '../../lib/types';

  let info = $state<AppInfo | null>(null);

  const credits: [string, string, string][] = [
    ['Tauri', 'MIT or Apache-2.0', 'Desktop application framework'],
    ['wry and tao', 'MIT or Apache-2.0', 'WebView and windowing libraries'],
    ['webview2-com', 'MIT', 'Rust bindings for Microsoft Edge WebView2'],
    ['Microsoft Edge WebView2', 'Microsoft software license', 'Web rendering engine, installed with Windows'],
    ['SQLite and rusqlite', 'Public domain and MIT', 'Storage for history, bookmarks and downloads'],
    ['Mozilla Readability', 'Apache-2.0', 'Article extraction for Reading mode'],
    ['Svelte', 'MIT', 'Interface framework'],
    ['Source Serif 4', 'SIL Open Font License 1.1', 'Display typeface by Frank Grießhammer, Adobe'],
    ['serde, tokio, url, rfd, base64, thiserror', 'MIT or Apache-2.0', 'Rust libraries'],
  ];

  onMount(() => {
    api.appInfo().then((i) => (info = i));
  });
</script>

<div class="hero">
  <Brand height={36} isDark={settings.dark} />
  <div>
    <div class="name">Silb Browser</div>
    <div class="ver">Version {info?.version ?? '…'}</div>
  </div>
</div>

<Group title="Runtime">
  <Row label="Silb" description={info?.version ?? ''} />
  <Row label="Tauri" description={info?.tauri ?? ''} />
  <Row label="Microsoft Edge WebView2" description={info?.webview2 || 'Unknown'} />
  <Row label="Platform" description={info ? `Windows (${info.arch})` : ''} />
  <Row label="Profile folder" description={info?.dataDir ?? ''} />
  <Row
    label="Updates"
    description="Silb doesn’t update itself yet. WebView2 is kept up to date by Windows, which delivers web engine security fixes."
  />
</Group>

<Group title="Open-source software">
  {#each credits as [name, license, role]}
    <Row label={name} description="{role}. {license}." />
  {/each}
</Group>

<style>
  .hero {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-bottom: 26px;
  }
  .name {
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 500;
  }
  .ver {
    color: var(--muted);
  }
</style>
