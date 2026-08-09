/**
 * What the composer offers you before you have picked anything.
 *
 * Shuffled, and drawn from both your own emoji and the site's popular ones.
 * The first version walked them in order — your history chronologically, then
 * popular — which replayed the same sequence on every pass and, if your history
 * was long enough to fill the cap, never reached the popular ones at all.
 */

/** how many to cycle through; long enough to feel varied, short enough to recur */
const MAX_SUGGESTIONS = 12;

/** neither source may take more than this, so both are always represented */
const PER_SOURCE = MAX_SUGGESTIONS / 2;

/** Fisher-Yates, on a copy. `random` is injectable so tests are deterministic. */
export function shuffle<T>(items: T[], random: () => number = Math.random): T[] {
  const out = [...items];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

export function buildSuggestions(opts: {
  /** emoji from your own recent statuses */
  recent?: string[];
  /** all-time popular across the site */
  popular?: string[];
  /** what the composer is showing right now — always first, so nothing jumps */
  current?: string;
  random?: () => number;
}): string[] {
  const random = opts.random ?? Math.random;
  const seen = new Set<string>();
  const clean = (list: string[] | undefined) => {
    const out: string[] = [];
    for (const raw of list ?? []) {
      const v = raw?.trim();
      if (!v || seen.has(v)) continue;
      seen.add(v);
      out.push(v);
    }
    return out;
  };

  const current = opts.current?.trim();
  if (current) seen.add(current);

  // Shuffle each source before taking its share, so a long history does not
  // always contribute the same half of itself.
  const recent = shuffle(clean(opts.recent), random).slice(0, PER_SOURCE);
  const popular = shuffle(clean(opts.popular), random).slice(0, PER_SOURCE);

  const rest = shuffle([...recent, ...popular], random).slice(
    0,
    MAX_SUGGESTIONS - (current ? 1 : 0),
  );
  return current ? [current, ...rest] : rest;
}
