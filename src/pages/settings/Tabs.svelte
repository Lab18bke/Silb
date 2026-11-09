<script lang="ts">
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { settings } from '../../lib/settings.svelte';

  const s = $derived(settings.value!);
</script>

<Group title="Opening links">
  <Row label="Open links in new tabs" description="When a site opens a link in a new window, open it as a tab in this window instead.">
    <Toggle label="Open links in new tabs" checked={s.openLinksInNewTab} onchange={(v) => settings.update({ openLinksInNewTab: v })} />
  </Row>
  <Row label="Switch to tabs opened in the background" description="Applies to Ctrl+click, middle-click and “Open link in new tab”.">
    <Toggle label="Switch to new tabs" checked={s.switchToNewTabs} onchange={(v) => settings.update({ switchToNewTabs: v })} />
  </Row>
</Group>

<Group title="Closing">
  <Row label="Ask before closing a window with several tabs">
    <Toggle label="Confirm before closing multiple tabs" checked={s.confirmCloseMultiple} onchange={(v) => settings.update({ confirmCloseMultiple: v })} />
  </Row>
  <Row label="Reopen closed tabs" description="Ctrl+Shift+T reopens up to 25 recently closed tabs in each window." />
</Group>

<Group title="Performance">
  <Row
    label="Save memory on inactive tabs"
    description="Tabs you haven’t looked at for 15 minutes are suspended. They resume instantly when you return. Tabs playing audio are never suspended."
  >
    <Toggle label="Save memory on inactive tabs" checked={s.suspendBackgroundTabs} onchange={(v) => settings.update({ suspendBackgroundTabs: v })} />
  </Row>
</Group>
