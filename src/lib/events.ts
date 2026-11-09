import { getCurrentWebview } from '@tauri-apps/api/webview';
import type { UnlistenFn } from '@tauri-apps/api/event';

export function on<T>(event: string, handler: (payload: T) => void): () => void {
  let unlisten: UnlistenFn | null = null;
  let disposed = false;
  getCurrentWebview()
    .listen<T>(event, (e) => handler(e.payload))
    .then((u) => {
      if (disposed) u();
      else unlisten = u;
    });
  return () => {
    disposed = true;
    unlisten?.();
  };
}

export const webviewLabel = getCurrentWebview().label;
export const isPrivateWindow = webviewLabel.startsWith('p');
