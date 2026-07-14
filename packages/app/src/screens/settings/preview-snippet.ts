export const PREVIEW_BEFORE: string[] = [
  "// Format a price for display",
  "export function formatPrice(cents: number) {",
  "  const amount = cents / 100;",
  '  return "$" + amount;',
  "}",
];

export const PREVIEW_AFTER: string[] = [
  "// Format a price for display",
  "export function formatPrice(cents: number): string {",
  "  const amount = cents / 100;",
  "  return `$${amount.toFixed(2)}`;",
  "}",
];

export const CHANGED_LINE_INDICES: ReadonlySet<number> = new Set([1, 3]);
