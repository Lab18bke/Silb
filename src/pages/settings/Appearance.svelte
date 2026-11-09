<script lang="ts">
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import Toggle from '../../components/Toggle.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { browser } from '../../chrome/state.svelte';
  import type { Settings } from '../../lib/types';

  const s = $derived(settings.value!);
  const accents: { id: Settings['accent']; label: string; color: string }[] = [
    { id: 'juniper', label: 'Juniper', color: '#2e6b5e' },
    { id: 'ink', label: 'Ink', color: '#34509a' },
    { id: 'clay', label: 'Clay', color: '#9c5536' },
    { id: 'plum', label: 'Plum', color: '#74467a' },
    { id: 'graphite', label: 'Graphite', color: '#4a4a46' },
  ];
</script>

<Group title="Theme">
  <Row label="Colour theme" description={browser.isPrivate ? 'Private windows always use their own dark theme.' : 'System follows your Windows setting. Websites that support it follow this choice too.'}>
    <Segmented
      label="Colour theme"
      value={s.theme}
      options={[
        { value: 'system', label: 'System', icon: 'monitor' },
        { value: 'light', label: 'Light', icon: 'sun' },
        { value: 'dark', label: 'Dark', icon: 'moon' },
      ]}
      onchange={(v) => settings.update({ theme: v })}
    />
  </Row>
  <Row label="Accent colour" description="Used for focus rings, selection and highlights.">
    <div class="swatches" role="radiogroup" aria-label="Accent colour">
      {#each accents as a}
        <button
          type="button"
          role="radio"
          aria-checked={s.accent === a.id}
          aria-label={a.label}
          title={a.label}
          style:--c={a.color}
          onclick={() => settings.update({ accent: a.id })}
        ></button>
      {/each}
    </div>
  </Row>
  <Row label="Density" description="Compact makes the tab strip and toolbar shorter.">
    <Segmented
      label="Density"
      value={s.density}
      options={[
        { value: 'comfortable', label: 'Comfortable' },
        { value: 'compact', label: 'Compact' },
      ]}
      onchange={(v) => settings.update({ density: v })}
    />
  </Row>
</Group>

<Group title="Layout">
  <Row label="Show bookmarks bar" description="Ctrl+Shift+B">
    <Toggle label="Show bookmarks bar" checked={s.showBookmarksBar} onchange={(v) => settings.update({ showBookmarksBar: v })} />
  </Row>
  <Row label="Show frequently visited sites on the start page">
    <Toggle label="Show frequently visited sites" checked={s.showTopSites} onchange={(v) => settings.update({ showTopSites: v })} />
  </Row>
</Group>

<style>
  .swatches {
    display: flex;
    gap: 10px;
  }
  .swatches button {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 0;
    padding: 0;
    background: var(--c);
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.12);
  }
  .swatches button[aria-checked='true'] {
    box-shadow: 0 0 0 2px var(--raised), 0 0 0 4px var(--c);
  }
</style>
