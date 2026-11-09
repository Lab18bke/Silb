<script lang="ts">
  import { onMount } from 'svelte';
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { api, errorText } from '../../lib/api';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../chrome/popover';
  import type { AppInfo } from '../../lib/types';

  const s = $derived(settings.value!);
  let info = $state<AppInfo | null>(null);

  async function pick() {
    try {
      const dir = await api.pickDownloadDir();
      if (dir) await settings.update({ downloadDir: dir });
    } catch (e) {
      toast(errorText(e));
    }
  }

  onMount(() => {
    api.appInfo().then((i) => (info = i));
  });
</script>

<Group title="Location">
  <Row label="Save files to" description={s.downloadDir || info?.downloadDir || ''}>
    {#if s.downloadDir}
      <button class="btn quiet" onclick={() => settings.update({ downloadDir: '' })}>Use default</button>
    {/if}
    <button class="btn" onclick={pick}>Change…</button>
  </Row>
  <Row label="Ask where to save each file" description="Shows a Save dialog for every download instead of saving to the folder above.">
    <Toggle label="Ask where to save each file" checked={s.askDownloadLocation} onchange={(v) => settings.update({ askDownloadLocation: v })} />
  </Row>
</Group>

<Group title="Behaviour">
  <Row label="Show the downloads panel when a download starts" description="Otherwise a dot appears on the downloads button.">
    <Toggle label="Show downloads panel" checked={s.showDownloadsPopup} onchange={(v) => settings.update({ showDownloadsPopup: v })} />
  </Row>
  <Row
    label="Existing files are never overwritten"
    description="If a file with the same name exists, Silb saves the new one as “name (1).ext”."
  />
</Group>
