/**
 * How the picker's grid grows.
 *
 * Pulled out of the component as a pure function for one reason: as reactive
 * code it read `visibleCount` and assigned it, and an effect that writes what it
 * reads re-runs itself until the update-depth limit kills the component — the
 * whole picker froze, and switching tabs stopped working with it. A function
 * that takes the state and returns the next step cannot do that, and can be
 * tested without a browser.
 */

export type GrowthState = {
  /** how many tiles are rendered right now */
  visibleCount: number;
  /** how many have been fetched */
  total: number;
  /** how many more to reveal at a time */
  windowStep: number;
  /** rendered tiles still waiting on an image */
  pending: number;
  /** how many images may be in flight before we stop adding more */
  maxInFlight: number;
  /** is the end of the grid on screen */
  sentinelVisible: boolean;
  /** is a fresh render or search in progress */
  loading: boolean;
  /** can the server still give us more */
  canFetchMore: boolean;
};

export type Growth =
  /** reveal more of what is already fetched */
  | { kind: "reveal"; visibleCount: number }
  /** everything fetched is on screen; ask for the next page */
  | { kind: "fetch" }
  /** do nothing */
  | { kind: "idle" };

/**
 * Reveal what is already fetched before asking for more, and do neither while a
 * batch is still loading — a grid of skeletons keeps the end of the list on
 * screen, and every pass would otherwise pile more requests onto a browser that
 * only runs a handful per host.
 */
export function nextGrowth(s: GrowthState): Growth {
  if (!s.sentinelVisible || s.loading) return { kind: "idle" };
  if (s.pending > s.maxInFlight) return { kind: "idle" };
  if (s.visibleCount < s.total) {
    return { kind: "reveal", visibleCount: Math.min(s.visibleCount + s.windowStep, s.total) };
  }
  return s.canFetchMore ? { kind: "fetch" } : { kind: "idle" };
}
