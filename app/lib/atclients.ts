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
  /** where to open a bluesky post, for clients that render the feed */
  postUrl?: (did: string, rkey: string) => string;
}

/** every appview client here uses bluesky's own post path shape */
const bskyStylePost = (base: string) => (did: string, rkey: string) =>
  `${base}/profile/${did}/post/${rkey}`;

const BSKY: AtClient = {
  value: "bsky",
  label: "bluesky",
  iconUrl: "https://web-cdn.bsky.app/static/apple-touch-icon.png",
  profileUrl: (h) => `https://bsky.app/profile/${h}`,
  postUrl: bskyStylePost("https://bsky.app"),
};

export const AT_CLIENTS: AtClient[] = [
  BSKY,
  {
    value: "blacksky",
    label: "blacksky",
    iconUrl: "https://blacksky.community/static/apple-touch-icon.png",
    profileUrl: (h) => `https://blacksky.community/profile/${h}`,
    postUrl: bskyStylePost("https://blacksky.community"),
  },
  {
    value: "witchsky",
    label: "witchsky",
    iconUrl: "https://witchsky.app/favicon.ico",
    profileUrl: (h) => `https://witchsky.app/profile/${h}`,
    postUrl: bskyStylePost("https://witchsky.app"),
  },
  {
    value: "reddwarf",
    label: "red dwarf",
    iconUrl: "https://reddwarf.app/redstar.png",
    profileUrl: (h) => `https://reddwarf.app/profile/${h}`,
    postUrl: bskyStylePost("https://reddwarf.app"),
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

/**
 * Where to open a bluesky post in the reader's chosen client.
 *
 * A record-oriented client like pdsls has no post view, so it falls back to its
 * record url rather than being sent somewhere that does not exist.
 */
export function postLink(
  clientValue: string | null | undefined,
  did: string,
  rkey: string,
): string {
  const client = resolveClient(clientValue);
  if (client.postUrl) return client.postUrl(did, rkey);
  const uri = `${did}/app.bsky.feed.post/${rkey}`;
  return client.recordUrl?.(`at://${uri}`) ?? `https://pdsls.dev/at/${uri}`;
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
