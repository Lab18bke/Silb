import { api, type Rect } from '../lib/api';
import { on } from '../lib/events';

export type PopoverKind =
  | 'menu'
  | 'suggestions'
  | 'bookmark'
  | 'permission'
  | 'downloads'
  | 'siteinfo'
  | 'popups'
  | 'confirm'
  | 'toast';

interface Margin {
  top: number;
  side: number;
  bottom: number;
}

interface Options {
  focus?: boolean;
  onEvent?: (name: string, data: any) => void;
  onClose?: () => void;
  margin?: Partial<Margin>;
}

interface Current {
  id: number;
  kind: PopoverKind;
  rect: Rect;
  margin: Margin;
  focus: boolean;
  onEvent?: (name: string, data: any) => void;
  onClose?: () => void;
}

let seq = 0;
let current: Current | null = null;
let lastClosed = { kind: null as PopoverKind | null, at: 0 };
let listeners: (() => void)[] = [];

export function initPopovers() {
  on<{ name: string; data: any }>('overlay-event', ({ name, data }) => {
    if (current && data?.id === current.id) current.onEvent?.(name, data);
  });
  on('overlay-hidden', () => {
    const c = current;
    if (c) lastClosed = { kind: c.kind, at: performance.now() };
    current = null;
    notify();
    c?.onClose?.();
  });
}

function notify() {
  for (const l of listeners) l();
}

export function onPopoverChange(fn: () => void) {
  listeners.push(fn);
  return () => (listeners = listeners.filter((l) => l !== fn));
}

function send(c: Current, data: unknown) {
  const { rect, margin } = c;
  const outer = {
    x: rect.x - margin.side,
    y: rect.y - margin.top,
    width: rect.width + margin.side * 2,
    height: rect.height + margin.top + margin.bottom,
  };
  return api.overlayShow(outer, { id: c.id, kind: c.kind, data, margin }, c.focus);
}

export function openPopover(kind: PopoverKind, rect: Rect, data: unknown, opts: Options = {}) {
  const previous = current;
  current = {
    id: ++seq,
    kind,
    rect,
    margin: { top: 16, side: 22, bottom: 30, ...opts.margin },
    focus: opts.focus ?? true,
    onEvent: opts.onEvent,
    onClose: opts.onClose,
  };
  previous?.onClose?.();
  notify();
  return send(current, data);
}

export function updatePopover(kind: PopoverKind, data: unknown, rect?: Rect) {
  if (!current || current.kind !== kind) return;
  if (rect) current.rect = rect;
  return send(current, data);
}

export function closePopover(kind?: PopoverKind) {
  if (!current || (kind && current.kind !== kind)) return;
  api.overlayHide();
}

export function justClosed(kind: PopoverKind, ms = 350) {
  return lastClosed.kind === kind && performance.now() - lastClosed.at < ms;
}

export function popoverKind(): PopoverKind | null {
  return current?.kind ?? null;
}

export function anchorBelow(el: Element, width: number, height: number, align: 'start' | 'end' = 'start', gap = 6): Rect {
  const r = el.getBoundingClientRect();
  const x = align === 'end' ? r.right - width : r.left;
  return { x: Math.max(8, Math.min(x, window.innerWidth - width - 8)), y: r.bottom + gap, width, height };
}

export function toast(message: string) {
  if (current && current.kind !== 'toast') return;
  const width = Math.min(420, Math.max(220, message.length * 7 + 48));
  const top = document.querySelector('header')?.getBoundingClientRect().bottom ?? 90;
  openPopover('toast', { x: (window.innerWidth - width) / 2, y: top + 10, width, height: 40 }, { message }, { focus: false });
}
