<script lang="ts">
  import { gifThumbUrl, gifBlobUrl, resolvePdsForDid } from '$lib/utils/gifdex'

  let {
    did,
    blobCid,
    pds,
    animated = false,
    alt = '',
    title,
    onsettled,
  }: {
    did: string
    blobCid: string
    /** owning PDS; only needed to animate, and to fall back if the CDN misses */
    pds?: string
    /** false renders the tiny static thumbnail — use it for grids */
    animated?: boolean
    alt?: string
    title?: string
    onsettled?: () => void
  } = $props()

  // The thumbnail is bsky's CDN serving a blob from a PDS it does not own, which
  // works but is not promised. On error, fall through to the real blob.
  let failed = $state(false)
  // resolved lazily so callers can just say `animated` without knowing the PDS
  let resolvedPds: string | null = $state(null)

  let host = $derived(pds ?? resolvedPds)

  $effect(() => {
    if ((animated || failed) && !pds && !resolvedPds) {
      resolvePdsForDid(did).then((p) => { resolvedPds = p })
    }
  })

  // shows the still thumbnail until the PDS is known, then swaps to the real
  // gif — a static frame is a better wait than an empty box.
  let src = $derived(
    (animated || failed) && host ? gifBlobUrl(host, did, blobCid) : gifThumbUrl(did, blobCid),
  )
</script>

<img
  {src}
  {alt}
  {title}
  loading="lazy"
  decoding="async"
  onload={() => onsettled?.()}
  onerror={() => {
    if (!failed && pds) failed = true
    else onsettled?.()
  }}
/>
