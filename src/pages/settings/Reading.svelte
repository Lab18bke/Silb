<script lang="ts">
  import Group from './Group.svelte';
  import Row from './Row.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import { settings } from '../../lib/settings.svelte';

  const s = $derived(settings.value!);
  const widths = [
    { value: '560', label: 'Narrow' },
    { value: '680', label: 'Medium' },
    { value: '840', label: 'Wide' },
  ];
  const nearest = $derived(widths.reduce((a, b) => (Math.abs(+b.value - s.readerWidth) < Math.abs(+a.value - s.readerWidth) ? b : a)).value);
</script>

<Group title="Reading mode">
  <Row label="Text size" description="You can also change it from the Reading mode toolbar.">
    <div class="size">
      <button class="btn" aria-label="Smaller" onclick={() => settings.update({ readerFontSize: Math.max(14, s.readerFontSize - 1) })}>A−</button>
      <span>{s.readerFontSize}px</span>
      <button class="btn" aria-label="Larger" onclick={() => settings.update({ readerFontSize: Math.min(30, s.readerFontSize + 1) })}>A+</button>
    </div>
  </Row>
  <Row label="Line width">
    <Segmented label="Line width" value={nearest} options={widths} onchange={(v) => settings.update({ readerWidth: Number(v) })} />
  </Row>
  <Row label="Typeface">
    <Segmented
      label="Typeface"
      value={s.readerFont}
      options={[
        { value: 'serif', label: 'Serif' },
        { value: 'sans', label: 'Sans serif' },
      ]}
      onchange={(v) => settings.update({ readerFont: v })}
    />
  </Row>
  <Row label="Theme" description="Auto follows the browser’s light or dark theme.">
    <Segmented
      label="Reading theme"
      value={s.readerTheme}
      options={[
        { value: 'auto', label: 'Auto' },
        { value: 'light', label: 'Light' },
        { value: 'sepia', label: 'Sepia' },
        { value: 'dark', label: 'Dark' },
      ]}
      onchange={(v) => settings.update({ readerTheme: v })}
    />
  </Row>
</Group>

<div
  class="preview"
  data-theme={s.readerTheme === 'auto' ? (settings.dark ? 'dark' : 'light') : s.readerTheme}
  class:sans={s.readerFont === 'sans'}
  style:--size="{s.readerFontSize}px"
  style:--width="{s.readerWidth}px"
>
  <h4>Preview</h4>
  <p>
    Reading mode keeps the article and drops the rest: navigation, sidebars and ads. Headings, images and links stay where
    they belong, and the text is set at a comfortable size and line length.
  </p>
</div>

<style>
  .size {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .size span {
    min-width: 40px;
    text-align: center;
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .preview {
    margin-top: 24px;
    padding: 24px 28px;
    border-radius: var(--radius-l);
    border: 1px solid var(--line);
    background: #fbfaf7;
    color: #24221f;
    font-family: 'Sitka Text', Georgia, serif;
  }
  .preview[data-theme='sepia'] {
    background: #f4ecd8;
    color: #45392a;
  }
  .preview[data-theme='dark'] {
    background: #1e1d1b;
    color: #e6e3dc;
  }
  .preview.sans {
    font-family: var(--font-ui);
  }
  .preview h4 {
    margin: 0 0 8px;
    font-size: 12px;
    opacity: 0.6;
    font-family: var(--font-ui);
    font-weight: 600;
  }
  .preview p {
    max-width: var(--width);
    font-size: var(--size);
    line-height: 1.7;
    margin: 0;
  }
</style>
