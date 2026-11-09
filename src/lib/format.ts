export function formatBytes(n: number | null | undefined): string {
  if (n == null || n < 0) return '';
  if (n < 1024) return `${n} B`;
  const units = ['KB', 'MB', 'GB', 'TB'];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
}

export function hostOf(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
}

export function displayUrl(url: string): string {
  if (!url) return '';
  if (url.startsWith('https://')) {
    const rest = url.slice(8);
    return rest.endsWith('/') && rest.indexOf('/') === rest.length - 1 ? rest.slice(0, -1) : rest;
  }
  return url;
}

export function timeOfDay(ts: number): string {
  return new Date(ts).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
}

export function dayLabel(ts: number): string {
  const d = new Date(ts);
  const today = new Date();
  const start = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((start(today) - start(d)) / 86400000);
  if (diff === 0) return 'Today';
  if (diff === 1) return 'Yesterday';
  return d.toLocaleDateString(undefined, {
    weekday: 'long',
    month: 'long',
    day: 'numeric',
    year: d.getFullYear() === today.getFullYear() ? undefined : 'numeric',
  });
}

export function greeting(): string {
  const h = new Date().getHours();
  if (h < 5) return 'Good night';
  if (h < 12) return 'Good morning';
  if (h < 18) return 'Good afternoon';
  return 'Good evening';
}

export const PERMISSION_LABELS: Record<string, string> = {
  location: 'Location',
  camera: 'Camera',
  microphone: 'Microphone',
  notifications: 'Notifications',
  clipboard: 'Clipboard',
  'automatic-downloads': 'Automatic downloads',
  'file-editing': 'File editing',
  'local-fonts': 'Fonts on your device',
  midi: 'MIDI devices',
  popups: 'Pop-ups and redirects',
  javascript: 'JavaScript',
};

export const PERMISSION_ASKS: Record<string, string> = {
  location: 'know your location',
  camera: 'use your camera',
  microphone: 'use your microphone',
  notifications: 'show notifications',
  clipboard: 'see text and images copied to the clipboard',
  'automatic-downloads': 'download multiple files',
  'file-editing': 'edit files on your device',
  'local-fonts': 'use fonts installed on your device',
  midi: 'control MIDI devices',
};
