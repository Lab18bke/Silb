function __silbReaderOpen(opts) {
  if (window.__silbReaderApply && document.querySelector('silb-reader')) {
    window.__silbReaderApply(opts);
    return { ok: true };
  }

  var article;
  try {
    var clone = document.cloneNode(true);
    article = new Readability(clone, {
      serializer: function (el) { return el; },
      charThreshold: 250
    }).parse();
  } catch (e) {
    return { ok: false, error: String((e && e.message) || e) };
  }
  if (!article || !article.content) return { ok: false, error: 'no-article' };
  var text = (article.content.textContent || '').trim();
  if (text.length < 140) return { ok: false, error: 'no-article' };

  var WIDTHS = [560, 680, 840];
  var THEMES = ['light', 'sepia', 'dark'];
  var state = {
    fontSize: opts.fontSize || 19,
    width: opts.width || 680,
    theme: opts.theme || 'auto',
    font: opts.font || 'serif'
  };

  var CSS = [
    ':host { all: initial; }',
    '.root { position: absolute; inset: 0; font-size: 16px; color: var(--fg); background: var(--bg);',
    '  --fg: #24221f; --muted: #7a756c; --bg: #fbfaf7; --rule: #e7e3db; --link: #2f6f62; --chip: #efece5;',
    '  -webkit-font-smoothing: antialiased; }',
    '.root[data-theme="sepia"] { --fg: #45392a; --muted: #85755e; --bg: #f4ecd8; --rule: #e2d6bb; --link: #8a5a2b; --chip: #ebe1c8; }',
    '.root[data-theme="dark"] { --fg: #e6e3dc; --muted: #9c978e; --bg: #1e1d1b; --rule: #34322e; --link: #8cc4b4; --chip: #2a2926; }',
    '.scroller { position: absolute; inset: 0; overflow-y: auto; outline: none; scroll-behavior: smooth; }',
    '.bar { position: sticky; top: 0; z-index: 2; display: flex; align-items: center; justify-content: space-between;',
    '  gap: 12px; padding: 10px 20px; background: color-mix(in srgb, var(--bg) 88%, transparent);',
    '  backdrop-filter: blur(10px); border-bottom: 1px solid transparent; transition: border-color .2s;',
    '  font: 13px/1.2 "Segoe UI", system-ui, sans-serif; }',
    '.bar.scrolled { border-bottom-color: var(--rule); }',
    '.site { color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 40%; }',
    '.tools { display: flex; align-items: center; gap: 4px; }',
    '.sep { width: 1px; height: 18px; background: var(--rule); margin: 0 6px; }',
    'button { all: unset; box-sizing: border-box; height: 30px; min-width: 30px; padding: 0 8px; border-radius: 8px;',
    '  display: inline-flex; align-items: center; justify-content: center; gap: 6px; cursor: pointer;',
    '  color: var(--fg); font: inherit; transition: background .15s; }',
    'button:hover { background: var(--chip); }',
    'button:focus-visible { outline: 2px solid var(--link); outline-offset: 1px; }',
    'button[aria-pressed="true"] { background: var(--chip); }',
    '.swatch { width: 16px; height: 16px; border-radius: 50%; border: 1px solid rgba(127,127,127,.45); }',
    '.swatch.light { background: #fbfaf7; } .swatch.sepia { background: #f4ecd8; } .swatch.dark { background: #1e1d1b; }',
    'svg { width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 1.5; stroke-linecap: round; stroke-linejoin: round; }',
    '.small { font-size: 12px; } .large { font-size: 16px; }',
    'article { box-sizing: border-box; max-width: calc(var(--width) * 1px + 48px); margin: 0 auto; padding: 40px 24px 120px;',
    '  font-size: calc(var(--size) * 1px); line-height: 1.7; }',
    '.root[data-font="serif"] article { font-family: "Sitka Text", Georgia, Cambria, "Times New Roman", serif; }',
    '.root[data-font="sans"] article { font-family: "Segoe UI", system-ui, sans-serif; line-height: 1.65; }',
    'h1.title { font-family: "Sitka Display", "Sitka Text", Georgia, serif; font-weight: 600; font-size: 2.05em; line-height: 1.18; letter-spacing: -0.01em; margin: 0 0 16px; }',
    '.meta { font: 13px/1.5 "Segoe UI", system-ui, sans-serif; color: var(--muted); padding-bottom: 28px; margin-bottom: 32px; border-bottom: 1px solid var(--rule); }',
    '.content h1, .content h2, .content h3, .content h4 { line-height: 1.3; margin: 1.8em 0 .6em; font-weight: 600; }',
    '.content h1 { font-size: 1.5em; } .content h2 { font-size: 1.32em; } .content h3 { font-size: 1.15em; } .content h4 { font-size: 1em; }',
    '.content p { margin: 0 0 1.15em; }',
    '.content a { color: var(--link); text-decoration-thickness: 1px; text-underline-offset: 2px; }',
    '.content img, .content video, .content picture, .content svg, .content iframe { max-width: 100%; height: auto; border-radius: 6px; }',
    '.content svg { width: auto; stroke: none; fill: currentColor; }',
    '.content figure { margin: 1.8em 0; } .content figcaption { font-size: .8em; color: var(--muted); margin-top: .6em; text-align: center; }',
    '.content blockquote { margin: 1.4em 0; padding: .2em 0 .2em 1.1em; border-left: 3px solid var(--rule); color: var(--muted); font-style: italic; }',
    '.content pre { background: var(--chip); padding: 14px 16px; border-radius: 8px; overflow-x: auto; font-size: .82em; line-height: 1.55; }',
    '.content code { font-family: "Cascadia Code", Consolas, monospace; font-size: .88em; background: var(--chip); padding: .1em .3em; border-radius: 4px; }',
    '.content pre code { background: none; padding: 0; }',
    '.content ul, .content ol { padding-left: 1.4em; margin: 0 0 1.15em; } .content li { margin: .3em 0; }',
    '.content table { border-collapse: collapse; width: 100%; font-size: .9em; margin: 1.4em 0; display: block; overflow-x: auto; }',
    '.content th, .content td { border: 1px solid var(--rule); padding: 6px 10px; text-align: left; }',
    '.content hr { border: 0; border-top: 1px solid var(--rule); margin: 2em 0; }'
  ].join('\n');

  function el(tag, cls, txt) {
    var n = document.createElement(tag);
    if (cls) n.className = cls;
    if (txt) n.textContent = txt;
    return n;
  }

  function icon(paths) {
    var svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('viewBox', '0 0 16 16');
    paths.forEach(function (d) {
      var p = document.createElementNS('http://www.w3.org/2000/svg', 'path');
      p.setAttribute('d', d);
      svg.appendChild(p);
    });
    return svg;
  }

  function button(label, title, onClick) {
    var b = el('button');
    if (typeof label === 'string') b.textContent = label; else b.appendChild(label);
    b.title = title;
    b.setAttribute('aria-label', title);
    b.addEventListener('click', onClick);
    return b;
  }

  function post(msg) {
    try {
      if (window.chrome && window.chrome.webview) window.chrome.webview.postMessage({ silbReader: msg });
    } catch (e) {}
  }

  var host = document.createElement('silb-reader');
  host.style.cssText = 'all: initial; position: fixed; inset: 0; z-index: 2147483647; display: block;';
  var shadow = host.attachShadow({ mode: 'closed' });
  var sheet = new CSSStyleSheet();
  sheet.replaceSync(CSS);
  shadow.adoptedStyleSheets = [sheet];

  var root = el('div', 'root');
  var scroller = el('div', 'scroller');
  scroller.tabIndex = -1;
  var bar = el('div', 'bar');
  var siteName = article.siteName || location.hostname.replace(/^www\./, '');
  bar.appendChild(el('div', 'site', siteName));

  var tools = el('div', 'tools');
  var smaller = el('span', 'small', 'A');
  var larger = el('span', 'large', 'A');
  tools.appendChild(button(smaller, 'Smaller text', function () { setPrefs({ fontSize: Math.max(14, state.fontSize - 1) }); }));
  tools.appendChild(button(larger, 'Larger text', function () { setPrefs({ fontSize: Math.min(30, state.fontSize + 1) }); }));
  tools.appendChild(el('span', 'sep'));
  var widthBtn = button(icon(['M2 4h12', 'M4 8h8', 'M2 12h12']), 'Content width', function () {
    var i = WIDTHS.indexOf(nearestWidth(state.width));
    setPrefs({ width: WIDTHS[(i + 1) % WIDTHS.length] });
  });
  tools.appendChild(widthBtn);
  var fontBtn = button('Aa', 'Switch typeface', function () {
    setPrefs({ font: state.font === 'serif' ? 'sans' : 'serif' });
  });
  tools.appendChild(fontBtn);
  tools.appendChild(el('span', 'sep'));
  var swatches = THEMES.map(function (t) {
    var b = button(el('span', 'swatch ' + t), t.charAt(0).toUpperCase() + t.slice(1) + ' theme', function () { setPrefs({ theme: t }); });
    b.dataset.theme = t;
    tools.appendChild(b);
    return b;
  });
  tools.appendChild(el('span', 'sep'));
  tools.appendChild(button(icon(['M4 4l8 8', 'M12 4l-8 8']), 'Exit Reading mode (Esc)', function () { close(true); }));
  bar.appendChild(tools);

  var art = el('article');
  art.appendChild(el('h1', 'title', article.title || document.title));
  var words = text.split(/\s+/).length;
  var meta = [article.byline, Math.max(1, Math.round(words / 230)) + ' min read'].filter(Boolean).join(' · ');
  art.appendChild(el('div', 'meta', meta));
  var content = el('div', 'content');
  content.appendChild(document.importNode(article.content, true));
  art.appendChild(content);

  scroller.appendChild(bar);
  scroller.appendChild(art);
  root.appendChild(scroller);
  shadow.appendChild(root);

  scroller.addEventListener('scroll', function () {
    bar.classList.toggle('scrolled', scroller.scrollTop > 4);
  }, { passive: true });

  function nearestWidth(w) {
    return WIDTHS.reduce(function (a, b) { return Math.abs(b - w) < Math.abs(a - w) ? b : a; });
  }

  function resolvedTheme() {
    if (state.theme !== 'auto') return state.theme;
    return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  }

  function apply() {
    root.style.setProperty('--size', String(state.fontSize));
    root.style.setProperty('--width', String(state.width));
    root.dataset.theme = resolvedTheme();
    root.dataset.font = state.font;
    swatches.forEach(function (b) { b.setAttribute('aria-pressed', String(b.dataset.theme === root.dataset.theme)); });
  }

  function setPrefs(p) {
    for (var k in p) state[k] = p[k];
    apply();
    post({ prefs: { fontSize: state.fontSize, width: state.width, theme: state.theme, font: state.font } });
  }

  var html = document.documentElement;
  var prevHtml = html.style.overflow;
  var prevBody = document.body ? document.body.style.overflow : '';

  function onKey(e) {
    if (e.key === 'Escape') {
      e.stopImmediatePropagation();
      e.preventDefault();
      close(true);
    }
  }

  function close(notify) {
    window.removeEventListener('keydown', onKey, true);
    host.remove();
    html.style.overflow = prevHtml;
    if (document.body) document.body.style.overflow = prevBody;
    delete window.__silbReaderApply;
    delete window.__silbReaderClose;
    if (notify) post({ closed: true });
  }

  window.__silbReaderApply = function (o) {
    for (var k in o) state[k] = o[k];
    apply();
  };
  window.__silbReaderClose = function () { close(false); };
  window.addEventListener('keydown', onKey, true);

  apply();
  html.appendChild(host);
  html.style.overflow = 'hidden';
  if (document.body) document.body.style.overflow = 'hidden';
  scroller.focus({ preventScroll: true });
  return { ok: true };
}
