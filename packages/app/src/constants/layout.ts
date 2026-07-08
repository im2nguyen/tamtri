export const MAX_CONTENT_WIDTH = 820;
export const SIDEBAR_WIDTH = 300;
export const COMPACT_BREAKPOINT = 768;

export function isCompact(width: number): boolean {
  return width < COMPACT_BREAKPOINT;
}

export function isDesktopHost(): boolean {
  return typeof window !== "undefined" && Boolean(window.tamtri?.transport);
}
