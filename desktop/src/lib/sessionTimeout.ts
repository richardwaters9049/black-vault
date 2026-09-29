/** Wall-clock inactivity tracking: delayed timers (for example after system sleep) still expire. */
export const DEFAULT_IDLE_MS = 5 * 60 * 1000;
export function hasSessionExpired(lastActivity: number, now: number, limit = DEFAULT_IDLE_MS): boolean {
  return !Number.isFinite(lastActivity) || !Number.isFinite(now) || now < lastActivity || now - lastActivity >= limit;
}
