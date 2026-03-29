import { views } from "$hatk";
import type { StatusRecord, StatusView } from "$hatk";
import type { BaseContext, Row } from "$hatk";

export async function hydrateStatuses(
  ctx: BaseContext,
  items: Row<StatusRecord>[],
): Promise<StatusView[]> {
  const now = new Date();

  return items.map((item) => {
    const expiresDate = item.value.expires ? new Date(item.value.expires) : null;

    return views.statusView({
      uri: item.uri,
      cid: item.cid,
      did: item.did,
      handle: item.handle ?? item.did,
      emoji: item.value.emoji,
      text: item.value.text,
      expires: item.value.expires,
      createdAt: item.value.createdAt,
      indexedAt: item.indexed_at ?? item.value.createdAt,
      expired: expiresDate ? expiresDate < now : false,
    });
  });
}
