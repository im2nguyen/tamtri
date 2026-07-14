/** Desktop shell helpers exposed by Electron preload (optional). */

export interface TamtriShellBridge {
  pickOpenFile?: (options?: {
    title?: string;
    filters?: { name: string; extensions: string[] }[];
  }) => Promise<string | null>;
  pickSaveFile?: (options?: {
    title?: string;
    defaultPath?: string;
    filters?: { name: string; extensions: string[] }[];
  }) => Promise<string | null>;
  showItemInFolder?: (path: string) => Promise<void>;
}

export function shellBridge(): TamtriShellBridge | undefined {
  if (typeof window === "undefined") return undefined;
  return window.tamtri?.shell;
}

export function electronFilePath(file: File): string | undefined {
  const candidate = file as File & { path?: string };
  return candidate.path && candidate.path.length > 0 ? candidate.path : undefined;
}
