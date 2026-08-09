/**
 * What the composer offers you before you have picked anything.
 *
 * Yours first, then what the whole site is using — your own habits are the
 * better guess, but a new account has none, so the popular feed backfills.
 */

/** how many to cycle through; long enough to feel varied, short enough to recur */
const MAX_SUGGESTIONS = 12;

export function buildSuggestions(opts: {
  /** emoji from your own recent statuses, most recent first */
  recent?: string[];
  /** all-time popular across the site */
  popular?: string[];
  /** what the composer is showing right now — always first, so nothing jumps */
  current?: string;
}): string[] {
  const out: string[] = [];
  const seen = new Set<string>();

  const push = (value: string | undefined) => {
    const v = value?.trim();
    if (!v || seen.has(v)) return;
    seen.add(v);
    out.push(v);
  };

  push(opts.current);
  for (const e of opts.recent ?? []) push(e);
  for (const e of opts.popular ?? []) push(e);

  return out.slice(0, MAX_SUGGESTIONS);
}
