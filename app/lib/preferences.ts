import { writable, get } from "svelte/store";
import { callXrpc } from "$hatk/client";

export interface Preferences {
  accentColor: string;
  font: string;
  theme: string;
}

const DEFAULT: Preferences = {
  accentColor: "#4a9eff",
  font: "mono",
  theme: "dark",
};

export const FONTS = [
  { value: "system", label: "system", css: "system-ui, -apple-system, sans-serif" },
  { value: "mono", label: "mono", css: "ui-monospace, SF Mono, Monaco, monospace" },
  { value: "serif", label: "serif", css: "ui-serif, Georgia, serif" },
  { value: "comic", label: "comic", css: "Comic Sans MS, Comic Sans, cursive" },
];

export const ACCENT_COLORS = [
  "#4a9eff", "#10b981", "#f59e0b", "#ef4444",
  "#8b5cf6", "#ec4899", "#06b6d4", "#f97316",
];

export const preferences = writable<Preferences>({ ...DEFAULT });

export function loadPreferences(prefs: Record<string, unknown> | null): void {
  if (!prefs) return;
  const merged = { ...DEFAULT };
  if (typeof prefs.accentColor === "string") merged.accentColor = prefs.accentColor;
  if (typeof prefs.font === "string") merged.font = prefs.font;
  if (typeof prefs.theme === "string") merged.theme = prefs.theme;
  preferences.set(merged);
  applyPreferences(merged);
}

export function applyPreferences(prefs: Preferences): void {
  if (typeof document === "undefined") return;
  document.documentElement.style.setProperty("--accent", prefs.accentColor);
  const font = FONTS.find((f) => f.value === prefs.font) ?? FONTS[1];
  document.documentElement.style.setProperty("--font-family", font.css);
  document.documentElement.setAttribute("data-theme", prefs.theme);
  localStorage.setItem("theme", prefs.theme);
}

export async function savePreferences(prefs: Preferences): Promise<void> {
  preferences.set(prefs);
  applyPreferences(prefs);
  await Promise.all([
    callXrpc("dev.hatk.putPreference", { key: "accentColor", value: prefs.accentColor }),
    callXrpc("dev.hatk.putPreference", { key: "font", value: prefs.font }),
    callXrpc("dev.hatk.putPreference", { key: "theme", value: prefs.theme }),
  ]);
}
