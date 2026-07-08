/**
 * Electron preload (structural stub). Exposes a minimal, typed `window.tamtri`
 * bridge to the renderer: invoke(command, args), event subscription, window
 * controls, and the local daemon transport. No Node globals leak to the page.
 */

export interface TamtriDesktopBridge {
  invoke(command: string, args?: unknown): Promise<unknown>;
  on(event: string, handler: (payload: unknown) => void): () => void;
}
