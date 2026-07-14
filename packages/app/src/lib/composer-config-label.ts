export function buildConfigTriggerLabel(
  harnessLabel: string,
  modelLabel: string,
  mode?: string | null,
): string {
  const base = `${harnessLabel} · ${modelLabel}`;
  if (!mode) return base;
  return `${base} · ${mode.charAt(0).toUpperCase()}${mode.slice(1)}`;
}
