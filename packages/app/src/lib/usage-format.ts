export function formatResetCountdown(resetsAt?: string | null): string | null {
  if (!resetsAt) return null;
  const target = Date.parse(resetsAt);
  if (!Number.isFinite(target)) return null;
  const deltaMs = target - Date.now();
  if (deltaMs <= 0) return "Resets soon";
  const totalMinutes = Math.ceil(deltaMs / 60_000);
  if (totalMinutes < 60) return `Resets in ${totalMinutes}m`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours < 24) {
    return minutes > 0 ? `Resets in ${hours}h ${minutes}m` : `Resets in ${hours}h`;
  }
  const days = Math.floor(hours / 24);
  const remHours = hours % 24;
  return remHours > 0 ? `Resets in ${days}d ${remHours}h` : `Resets in ${days}d`;
}
