/** Normalize expo-router paths so `/onboarding` and `/onboarding/` compare equal. */
export function normalizeRoutePath(path: string): string {
  const withoutQuery = path.split("?")[0] ?? path;
  if (withoutQuery.length > 1 && withoutQuery.endsWith("/")) {
    return withoutQuery.slice(0, -1);
  }
  return withoutQuery;
}

/** Reject malformed conversation slugs before they reach daemon UUID parsing. */
export function isConversationRouteId(value: unknown): value is string {
  return (
    typeof value === "string" &&
    /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(
      value,
    )
  );
}
