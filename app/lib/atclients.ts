import { browser } from "$app/environment";
import { writable } from "svelte/store";

// preferred atproto client — open profiles and records in the app of your choice.
// the registry mirrors the shared client list in plyr.fm / leaflet-search
// (@zzstoatzz.io); keep it in sync with those rather than inventing entries.
export interface AtClient {
  value: string;
  label: string;
  iconUrl: string;
  profileUrl: (handleOrDid: string) => string;
  recordUrl?: (atUri: string) => string;
}

const BSKY: AtClient = {
  value: "bsky",
  label: "bluesky",
  iconUrl: "https://web-cdn.bsky.app/static/apple-touch-icon.png",
  profileUrl: (h) => `https://bsky.app/profile/${h}`,
};

export const AT_CLIENTS: AtClient[] = [
  BSKY,
  {
    value: "blacksky",
    label: "blacksky",
    iconUrl: "https://blacksky.community/static/apple-touch-icon.png",
    profileUrl: (h) => `https://blacksky.community/profile/${h}`,
  },
  {
    value: "witchsky",
    label: "witchsky",
    iconUrl: "https://witchsky.app/favicon.ico",
    profileUrl: (h) => `https://witchsky.app/profile/${h}`,
  },
  {
    value: "reddwarf",
    label: "red dwarf",
    iconUrl: "https://reddwarf.app/redstar.png",
    profileUrl: (h) => `https://reddwarf.app/profile/${h}`,
  },
  {
    value: "pdsls",
    label: "pdsls",
    iconUrl: "https://pdsls.dev/favicon.ico",
    profileUrl: (h) => `https://pdsls.dev/at/${h}`,
    recordUrl: (uri) => `https://pdsls.dev/at/${uri.replace(/^at:\/\//, "")}`,
  },
];

export const DEFAULT_AT_CLIENT = BSKY.value;

const STORAGE_KEY = "atprotoClient";

export function resolveClient(value: string | null | undefined): AtClient {
  return AT_CLIENTS.find((c) => c.value === value) ?? BSKY;
}

function initialValue(): string {
  if (!browser) return DEFAULT_AT_CLIENT;
  return localStorage.getItem(STORAGE_KEY) ?? DEFAULT_AT_CLIENT;
}

export const atprotoClient = writable<string>(initialValue());

export function setPreferredClient(value: string): void {
  atprotoClient.set(value);
  if (browser) localStorage.setItem(STORAGE_KEY, value);
}
