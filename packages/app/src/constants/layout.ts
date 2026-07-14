export const MAX_CONTENT_WIDTH = 820;
export const CONVERSATION_COLUMN_WIDTH = 736;
export const SIDEBAR_WIDTH = 300;
export const ARTIFACT_SIDEBAR_WIDTH = 380;
export const ARTIFACT_SIDEBAR_INLINE_MIN = 1100;
export const COMPACT_BREAKPOINT = 768;

export const MIN_LEFT_SIDEBAR_WIDTH = 208;
export const MAX_LEFT_SIDEBAR_WIDTH = 480;
export const MIN_ARTIFACT_SIDEBAR_WIDTH = 280;
export const MAX_ARTIFACT_SIDEBAR_WIDTH = 600;
export const MIN_MAIN_CONTENT_WIDTH = 640;

function clampNumber(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export function clampLeftSidebarWidth(width: number, viewportWidth?: number): number {
  let max = MAX_LEFT_SIDEBAR_WIDTH;
  if (viewportWidth !== undefined) {
    max = Math.min(max, Math.max(MIN_LEFT_SIDEBAR_WIDTH, viewportWidth - MIN_MAIN_CONTENT_WIDTH));
  }
  return clampNumber(width, MIN_LEFT_SIDEBAR_WIDTH, max);
}

export function clampArtifactSidebarWidth(width: number, viewportWidth?: number): number {
  let max = MAX_ARTIFACT_SIDEBAR_WIDTH;
  if (viewportWidth !== undefined) {
    max = Math.min(max, Math.max(MIN_ARTIFACT_SIDEBAR_WIDTH, viewportWidth - MIN_MAIN_CONTENT_WIDTH));
  }
  return clampNumber(width, MIN_ARTIFACT_SIDEBAR_WIDTH, max);
}

export function isCompact(width: number): boolean {
  return width < COMPACT_BREAKPOINT;
}

export function isArtifactSidebarInline(width: number): boolean {
  return width >= ARTIFACT_SIDEBAR_INLINE_MIN;
}

export function isDesktopHost(): boolean {
  return typeof window !== "undefined" && Boolean(window.tamtri?.transport);
}
