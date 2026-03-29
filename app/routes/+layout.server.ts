import { callXrpc, parseViewer } from "$hatk/client";
import type { LayoutServerLoad } from "./$types";

export const load: LayoutServerLoad = async ({ cookies }) => {
  const viewer = await parseViewer(cookies);

  return {
    viewer,
    preferences: viewer
      ? callXrpc("dev.hatk.getPreferences").catch(() => null)
      : null,
  };
};
