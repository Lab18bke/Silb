const LINKS = {
  github: 'https://github.com/Lab18bke/Silb',
  releases: 'https://github.com/Lab18bke/Silb/releases',
  installer: 'https://github.com/Lab18bke/Silb/releases/download/Silb_Releases/Silb-Setup.exe',
  portable: 'https://github.com/Lab18bke/Silb/releases/download/Silb_Releases/Silb-Portable.exe',
};

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

const circle = (cx, cy, r) => `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0z`;

const ICONS = {
  window: ['M2.5 3.5h11v9h-11z', 'M2.5 6h11'],
  search: [circle(7, 7, 4.5), 'M10.4 10.4 13.5 13.5'],
  download: ['M8 2.5v7.5', 'M4.5 6.5 8 10l3.5-3.5', 'M3 13h10'],
  shield: ['M8 2l5 2v4c0 3-2.2 5.2-5 6-2.8-.8-5-3-5-6V4z', 'M5.8 8.2 7.3 9.7 10.4 6.4'],
  lock: ['M4.5 7h7a.5.5 0 0 1 .5.5v5a.5.5 0 0 1-.5.5h-7a.5.5 0 0 1-.5-.5v-5a.5.5 0 0 1 .5-.5z', 'M5.5 7V5a2.5 2.5 0 0 1 5 0v2'],
  bookmark: ['M4.5 2.5h7v11L8 11l-3.5 2.5z'],
  reader: ['M8 4.5C6.6 3.5 4.6 3 2.5 3v9.5c2.1 0 4.1.5 5.5 1.5 1.4-1 3.4-1.5 5.5-1.5V3C11.4 3 9.4 3.5 8 4.5z', 'M8 4.5V14'],
  find: [circle(6.5, 6.5, 4), 'M9.6 9.6 13.5 13.5', 'M4.8 6.5h3.4'],
  sun: [circle(8, 8, 2.5), 'M8 1.5v1.5', 'M8 13v1.5', 'M1.5 8H3', 'M13 8h1.5', 'M3.4 3.4l1.06 1.06', 'M11.54 11.54l1.06 1.06', 'M3.4 12.6l1.06-1.06', 'M11.54 4.46l1.06-1.06'],
  home: ['M2.5 7.5 8 3l5.5 4.5', 'M4 6.5v7h8v-7'],
  clock: [circle(8, 8, 6), 'M8 4.5V8l2.5 1.5'],
  retry: ['M3 8a5 5 0 1 0 1.46-3.54', 'M4 1.9v2.9h2.9'],
  settings: ['M3 5h6', 'M12 5h1', 'M3 11h1', 'M7 11h6', circle(10.5, 5, 1.5), circle(5.5, 11, 1.5)],
  mask: [
    'M1.8 7.4c2.2-.8 10.2-.8 12.4 0',
    'M4.7 7 5.6 3.7c.2-.6.7-.8 1.2-.5l1.2.6 1.2-.6c.5-.3 1 0 1.2.5l.9 3.3',
    circle(5.3, 11.3, 1.8),
    circle(10.7, 11.3, 1.8),
    'M7.1 11.1c.6-.4 1.2-.4 1.8 0',
  ],
  installer: ['M3 2.5h10v11H3z', 'M8 5v5', 'M5.8 8 8 10.2 10.2 8', 'M5.5 12h5'],
  portable: ['M4.5 2h7l1 2v9.5h-9V4z', 'M3.5 4h9', 'M6.5 7.5h3'],
};

let iconSeq = 0;

function glossyIcon(name, size) {
  const paths = ICONS[name];
  if (!paths) return '';
  const id = `gi${++iconSeq}`;
  const body = paths
    .map((d) => {
      const closed = /z\s*$/i.test(d);
      return `<path d="${d}"${closed ? ` class="glass" fill="url(#${id}b)"` : ''}/>`;
    })
    .join('');
  const sheen = paths.map((d) => `<path d="${d}"/>`).join('');
  return `<svg class="glossy" width="${size}" height="${size}" viewBox="0 0 16 16" aria-hidden="true">
<defs>
<linearGradient id="${id}b" gradientUnits="userSpaceOnUse" x1="0" y1="1.5" x2="0" y2="14.5"><stop offset="0" class="t"/><stop offset=".5" class="m"/><stop offset="1" class="b"/></linearGradient>
<linearGradient id="${id}f" gradientUnits="userSpaceOnUse" x1="0" y1="1" x2="0" y2="12"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#fff" stop-opacity="0"/></linearGradient>
<mask id="${id}m" maskUnits="userSpaceOnUse" x="-1" y="-1" width="18" height="18"><rect x="-1" y="-1" width="18" height="18" fill="url(#${id}f)"/></mask>
</defs>
<g fill="none" stroke="url(#${id}b)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">${body}</g>
<g class="sheen" mask="url(#${id}m)" fill="none" stroke-width=".5" stroke-linecap="round" stroke-linejoin="round" transform="translate(0 -.42)">${sheen}</g>
</svg>`;
}

function mountIcons() {
  document.querySelectorAll('[data-icon]').forEach((el) => {
    const box = el.getBoundingClientRect().width || 44;
    const size = Math.round(Math.min(30, Math.max(18, box * 0.48)));
    el.innerHTML = glossyIcon(el.dataset.icon, size);
  });
}

function setupMarquee() {
  document.querySelectorAll('[data-marquee]').forEach((track) => {
    const items = [...track.children];
    items.forEach((item) => {
      const copy = item.cloneNode(true);
      copy.setAttribute('aria-hidden', 'true');
      track.appendChild(copy);
    });
  });
}

function applyLinks() {
  document.querySelectorAll('[data-link]').forEach((a) => {
    const href = LINKS[a.dataset.link];
    if (!href) return;
    a.href = href;
    if (/^https?:/.test(href)) {
      a.target = '_blank';
      a.rel = 'noopener';
    }
  });
}

function setupNav() {
  const nav = document.querySelector('[data-nav]');
  const update = () => nav.classList.toggle('is-scrolled', window.scrollY > 12);
  update();
  window.addEventListener('scroll', update, { passive: true });
}

function setupHero() {
  const stage = document.querySelector('[data-stage]');
  const floats = [...document.querySelectorAll('[data-float]')];
  if (!stage) return;
  if (reduced) {
    stage.style.setProperty('--tilt', '0');
    return;
  }
  let mouseX = 0;
  let mouseY = 0;
  let ticking = false;

  const render = () => {
    ticking = false;
    const rect = stage.getBoundingClientRect();
    const vh = window.innerHeight;
    const progress = Math.min(1, Math.max(0, (vh - rect.top) / (vh * 0.9) - 0.15));
    const tilt = 1 - easeOut(progress);
    stage.style.setProperty('--tilt', tilt.toFixed(4));
    floats.forEach((f) => {
      const depth = parseFloat(f.dataset.float);
      const fy = (1 - tilt) * depth * -36 + mouseY * depth * 10;
      const fx = mouseX * depth * 14;
      f.style.setProperty('--fx', `${fx.toFixed(1)}px`);
      f.style.setProperty('--fy', `${fy.toFixed(1)}px`);
    });
  };

  const request = () => {
    if (!ticking) {
      ticking = true;
      requestAnimationFrame(render);
    }
  };

  window.addEventListener('scroll', request, { passive: true });
  window.addEventListener('resize', request);
  window.addEventListener(
    'pointermove',
    (e) => {
      if (e.pointerType !== 'mouse') return;
      mouseX = e.clientX / window.innerWidth - 0.5;
      mouseY = e.clientY / window.innerHeight - 0.5;
      request();
    },
    { passive: true },
  );
  render();

  const label = document.querySelector('[data-progress-label]');
  if (label) {
    const start = performance.now();
    const tick = (now) => {
      const t = ((now - start) % 5500) / 5500;
      const p = Math.min(1, easeOut(Math.min(1, t / 0.8)));
      const mb = 0.5 + p * 5.6;
      label.textContent = p >= 1 ? '6.1 MB, done' : `${mb.toFixed(1)} MB of 6.1 MB`;
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }
}

function easeOut(t) {
  return 1 - Math.pow(1 - t, 3);
}

function setupTour() {
  const steps = [...document.querySelectorAll('[data-step]')];
  const frames = [...document.querySelectorAll('.tour-frame img')];
  const rail = document.querySelector('[data-rail]');
  if (!steps.length) return;

  const activate = (index) => {
    steps.forEach((s, i) => s.classList.toggle('is-active', i === index));
    frames.forEach((f, i) => f.classList.toggle('is-active', i === index));
    if (rail) rail.style.setProperty('--rail', `${((index + 1) / steps.length) * 100}%`);
  };

  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) activate(Number(entry.target.dataset.step));
      });
    },
    { rootMargin: '-45% 0px -45% 0px', threshold: 0 },
  );
  steps.forEach((s) => observer.observe(s));
  activate(0);
}

function setupCompare() {
  const box = document.querySelector('[data-compare]');
  const range = document.querySelector('[data-compare-range]');
  if (!box || !range) return;

  const set = (value) => {
    const v = Math.min(100, Math.max(0, value));
    box.style.setProperty('--pos', `${v}%`);
    range.value = String(v);
  };

  range.addEventListener('input', () => set(Number(range.value)));

  let dragging = false;
  const fromEvent = (e) => {
    const rect = box.getBoundingClientRect();
    set(((e.clientX - rect.left) / rect.width) * 100);
  };
  box.addEventListener('pointerdown', (e) => {
    dragging = true;
    box.setPointerCapture(e.pointerId);
    fromEvent(e);
  });
  box.addEventListener('pointermove', (e) => dragging && fromEvent(e));
  box.addEventListener('pointerup', () => (dragging = false));
  box.addEventListener('pointercancel', () => (dragging = false));

  if (!reduced) {
    const io = new IntersectionObserver(
      (entries) => {
        if (!entries[0].isIntersecting) return;
        io.disconnect();
        const start = performance.now();
        const sweep = (now) => {
          if (dragging) return;
          const t = Math.min(1, (now - start) / 1600);
          set(50 + Math.sin(t * Math.PI * 2) * 22 * (1 - t));
          if (t < 1) requestAnimationFrame(sweep);
        };
        requestAnimationFrame(sweep);
      },
      { threshold: 0.6 },
    );
    io.observe(box);
  }
}

applyLinks();
setupMarquee();
mountIcons();
setupNav();
setupHero();
setupTour();
setupCompare();
