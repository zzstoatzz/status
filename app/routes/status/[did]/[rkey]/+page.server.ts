import { callXrpc } from "$hatk/client";
import { isCustomEmoji, customEmojiName, resolveBufoUrl } from "$lib/utils/emoji";
import { gifPreviewUrl } from "$lib/utils/gifdex";
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
    gif?: { uri: string; cid: string } | string;
  };
  gif?: { uri: string; cid: string } | string;
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
      const gif = status.value?.gif ?? status.gif;

      // A gif status previews as the gif itself. porxie serves it CID-verified
      // at a stable public url, and bluesky's card proxy passes gifs through
      // byte-for-byte — verified, including a 10MB one — so the preview can
      // actually animate rather than showing a still frame.
      const ogImage =
        gifPreviewUrl(gif) ??
        (emoji && isCustomEmoji(emoji)
          ? await resolveBufoUrl(customEmojiName(emoji), fetch)
          : null);

      return { did, rkey, status, ogImage, ogImageIsGif: Boolean(gifPreviewUrl(gif)) };
    }
  } catch {}

  return { did, rkey, status: null as StatusRecord | null, ogImage: null, ogImageIsGif: false };
};
