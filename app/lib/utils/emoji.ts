export function isCustomEmoji(emoji: string): boolean {
  return emoji?.startsWith("custom:") ?? false;
}

export function customEmojiName(emoji: string): string {
  return emoji.slice(7);
}

// client img src: one canonical url. find-bufo's /e/{name} owns resolution
// (its own static -> bufo.zone png -> bufo.zone gif), so consumers don't walk candidates.
export function bufoImageUrl(name: string): string {
  return `https://find-bufo.com/e/${name}.png`;
}

export function bufoFallbackUrl(name: string): string {
  return `https://all-the.bufo.zone/${name}.gif`;
}

// custom bufos (added directly to find-bufo, not scraped from bufo.zone) live under find-bufo's own static dir
export function bufoCustomUrl(name: string): string {
  return `https://find-bufo.com/static/${name}.png`;
}

// direct (non-redirecting) urls in priority order — used server-side (og/twitter images),
// where crawlers don't reliably follow the resolver's 302, so we resolve to a concrete url here.
const bufoCandidateUrls = (name: string): string[] => [
  `https://all-the.bufo.zone/${name}.png`,
  bufoFallbackUrl(name),
  bufoCustomUrl(name),
];

// walk candidate urls until one loads: bufo.zone png -> bufo.zone gif -> find-bufo custom png
export function handleBufoError(img: HTMLImageElement, name: string): void {
  const step = Number(img.dataset.bufoStep ?? "0");
  const fallbacks = bufoCandidateUrls(name).slice(1);
  if (step < fallbacks.length) {
    img.dataset.bufoStep = String(step + 1);
    img.src = fallbacks[step];
  } else {
    img.onerror = null;
  }
}

// resolve the first candidate url that actually exists — for server-rendered og/twitter images,
// where crawlers don't run the onerror fallback. falls back to the default url if none respond.
export async function resolveBufoUrl(
  name: string,
  fetchFn: typeof fetch = fetch,
): Promise<string> {
  const candidates = bufoCandidateUrls(name);
  for (const url of candidates) {
    try {
      const res = await fetchFn(url, { method: "HEAD" });
      if (res.ok) return url;
    } catch {
      // network error — try the next candidate
    }
  }
  return candidates[0];
}

export function parseLinks(text: string): string {
  if (!text) return "";
  const escaped = text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
  return escaped.replace(
    /\[([^\]]+)\]\(([^)]+)\)/g,
    (_match: string, linkText: string, url: string) => {
      if (url.startsWith("http://") || url.startsWith("https://")) {
        return `<a href="${url}" target="_blank" rel="noopener">${linkText}</a>`;
      }
      return _match;
    },
  );
}

export function parseStatusUri(uri: string): { did: string; rkey: string } {
  const parts = uri.split("/");
  return { did: parts[2], rkey: parts[parts.length - 1] };
}

let bufoListCache: string[] | null = null;

export async function loadBufoList(): Promise<string[]> {
  if (bufoListCache) return bufoListCache;
  const res = await fetch("/bufos.json");
  if (!res.ok) throw new Error("Failed to load bufos");
  bufoListCache = await res.json();
  return bufoListCache!;
}

export async function searchBufos(
  query: string,
  topK = 20,
): Promise<Array<{ name: string; score: number }>> {
  const params = new URLSearchParams({
    query,
    top_k: String(topK),
  });
  const res = await fetch(`https://find-bufo.fly.dev/api/search?${params}`);
  if (!res.ok) throw new Error("bufo search failed");
  const data = await res.json();
  return data.results;
}

let emojiDataCache: {
  emojis: Record<string, string[]>;
  categories: Record<string, string[]>;
} | null = null;

const DEFAULT_FREQUENT = [
  "😊", "👍", "❤️", "😂", "🎉", "🔥", "✨", "💯",
  "🚀", "💪", "🙏", "👏", "😴", "🤔", "👀", "💻",
];

export { DEFAULT_FREQUENT };

export async function loadEmojiData() {
  if (emojiDataCache) return emojiDataCache;
  const response = await fetch(
    "https://cdn.jsdelivr.net/npm/emoji-datasource@15.1.0/emoji.json",
  );
  if (!response.ok) throw new Error("Failed to fetch emoji data");
  const data = await response.json();

  const emojis: Record<string, string[]> = {};
  const categories: Record<string, string[]> = {
    people: [], nature: [], food: [], activity: [],
    travel: [], objects: [], symbols: [], flags: [],
  };
  const categoryMap: Record<string, string> = {
    "Smileys & Emotion": "people",
    "People & Body": "people",
    "Animals & Nature": "nature",
    "Food & Drink": "food",
    Activities: "activity",
    "Travel & Places": "travel",
    Objects: "objects",
    Symbols: "symbols",
    Flags: "flags",
  };

  for (const emoji of data) {
    const char = emoji.unified
      .split("-")
      .map((u: string) => String.fromCodePoint(parseInt(u, 16)))
      .join("");
    const keywords = [
      ...(emoji.short_names || []),
      ...(emoji.name ? emoji.name.toLowerCase().split(/[\s_-]+/) : []),
    ];
    emojis[char] = keywords;
    const cat = categoryMap[emoji.category];
    if (cat && categories[cat]) categories[cat].push(char);
  }

  emojiDataCache = { emojis, categories };
  return emojiDataCache;
}

export function searchEmojis(
  query: string,
  data: { emojis: Record<string, string[]> },
): string[] {
  if (!query) return [];
  const q = query.toLowerCase();
  return Object.entries(data.emojis)
    .filter(([, keywords]) => keywords.some((k) => k.includes(q)))
    .map(([char]) => char)
    .slice(0, 50);
}
