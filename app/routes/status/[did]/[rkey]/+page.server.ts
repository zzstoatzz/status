import { callXrpc } from "$hatk/client";
import { isCustomEmoji, customEmojiName, resolveBufoUrl } from "$lib/utils/emoji";
import type { PageServerLoad } from "./$types";

// getRecord may return the status fields flat or nested under `value`, so accept both
type StatusRecord = {
  emoji?: string;
  text?: string;
  handle?: string;
  createdAt?: string;
  expires?: string;
  value?: {
    emoji?: string;
    text?: string;
    handle?: string;
    createdAt?: string;
    expires?: string;
  };
};

export const load: PageServerLoad = async ({ params, fetch }) => {
  const did = decodeURIComponent(params.did);
  const rkey = decodeURIComponent(params.rkey);
  const uri = `at://${did}/io.zzstoatzz.status.record/${rkey}`;

  try {
    const res = await callXrpc("dev.hatk.getRecord", { uri });
    if (res.record) {
      const status = res.record as StatusRecord;
      const emoji = status.value?.emoji ?? status.emoji;
      const ogImage =
        emoji && isCustomEmoji(emoji)
          ? await resolveBufoUrl(customEmojiName(emoji), fetch)
          : null;
      return { did, rkey, status, ogImage };
    }
  } catch {}

  return { did, rkey, status: null as StatusRecord | null, ogImage: null };
};
