export const UI_DENSITIES = ["compact", "comfortable", "spacious"] as const;

export type UiDensity = (typeof UI_DENSITIES)[number];

export const DEFAULT_UI_DENSITY: UiDensity = "comfortable";

const DENSITY_SCALE: Record<UiDensity, number> = {
  compact: 0.85,
  comfortable: 1,
  spacious: 1.15,
};

export interface DensityTokens {
  scale: number;
  rowHeight: number;
  rowPaddingY: number;
  rowGap: number;
  chatGutter: number;
  chatGutterWide: number;
  composerPaddingX: number;
  composerPaddingTop: number;
  composerPaddingBottom: number;
  composerFooterPadding: number;
  settingsRowPaddingY: number;
}

export function isUiDensity(value: unknown): value is UiDensity {
  return typeof value === "string" && UI_DENSITIES.includes(value as UiDensity);
}

export function normalizeUiDensity(
  value: unknown,
  fallback: UiDensity = DEFAULT_UI_DENSITY,
): UiDensity {
  return isUiDensity(value) ? value : fallback;
}

/** Adds or repairs density while preserving every other persisted appearance field. */
export function migrateAppearanceDensityState(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return { density: DEFAULT_UI_DENSITY };
  }
  const state = value as Record<string, unknown>;
  return {
    ...state,
    density: normalizeUiDensity(state.density),
  };
}

export function deriveDensityTokens(density: UiDensity = DEFAULT_UI_DENSITY): DensityTokens {
  const scale = DENSITY_SCALE[density];
  const scaled = (value: number) => Math.round(value * scale * 100) / 100;

  return {
    scale,
    rowHeight: scaled(28),
    rowPaddingY: scaled(2),
    rowGap: scaled(8),
    chatGutter: scaled(12),
    chatGutterWide: scaled(20),
    composerPaddingX: scaled(12),
    composerPaddingTop: scaled(12),
    composerPaddingBottom: scaled(8),
    composerFooterPadding: scaled(6),
    settingsRowPaddingY: scaled(10),
  };
}
