import { describe, expect, it } from "vitest";
import { nextGrowth, type GrowthState } from "./grid.ts";

const base: GrowthState = {
  visibleCount: 12,
  total: 28,
  windowStep: 12,
  pending: 0,
  maxInFlight: 6,
  sentinelVisible: true,
  loading: false,
  canFetchMore: false,
};

const at = (over: Partial<GrowthState>) => nextGrowth({ ...base, ...over });

describe("nextGrowth", () => {
  it("reveals what is already fetched before asking for more", () => {
    expect(at({})).toEqual({ kind: "reveal", visibleCount: 24 });
  });

  it("never reveals past what has been fetched", () => {
    // searching "bunny" returns 28 with a window of 12: 12 -> 24 -> 28, not 36
    expect(at({ visibleCount: 24 })).toEqual({ kind: "reveal", visibleCount: 28 });
  });

  /**
   * The freeze. Applying the result must be a fixed point once everything is
   * shown — as an effect this step wrote the count it also read, so it re-ran
   * itself until svelte's update-depth limit tore the picker down.
   */
  it("settles instead of stepping forever", () => {
    let state = { ...base };
    for (let i = 0; i < 100; i++) {
      const g = nextGrowth(state);
      if (g.kind !== "reveal") break;
      expect(g.visibleCount).toBeGreaterThan(state.visibleCount);
      state = { ...state, visibleCount: g.visibleCount };
    }
    expect(state.visibleCount).toBe(28);
    expect(nextGrowth(state)).toEqual({ kind: "idle" });
  });

  it("asks for a page only once everything fetched is on screen", () => {
    expect(at({ visibleCount: 28, canFetchMore: true })).toEqual({ kind: "fetch" });
    // a search returns no cursor, so there is nothing more to ask for
    expect(at({ visibleCount: 28, canFetchMore: false })).toEqual({ kind: "idle" });
  });

  it("holds off while a batch of images is still loading", () => {
    // the runaway: 30 tiles in flight kept the sentinel on screen behind the
    // skeletons, and each pass queued another page
    expect(at({ pending: 7 })).toEqual({ kind: "idle" });
    expect(at({ pending: 6 })).toEqual({ kind: "reveal", visibleCount: 24 });
  });

  it("does nothing when the end of the grid is off screen or mid-render", () => {
    expect(at({ sentinelVisible: false })).toEqual({ kind: "idle" });
    expect(at({ loading: true })).toEqual({ kind: "idle" });
  });

  it("handles an empty result without proposing anything", () => {
    expect(at({ visibleCount: 12, total: 0 })).toEqual({ kind: "idle" });
  });
});
