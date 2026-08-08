<script lang="ts">
  import { gifRenditionUrl, type GifVariant } from '$lib/utils/gifdex'

  let {
    did,
    blobCid,
    source,
    variant = 'preview',
    alt = '',
    title,
    onsettled,
  }: {
    did: string
    blobCid: string
    /** owning gif source, so its CDN is preferred over the blob proxy */
    source: string
    variant?: GifVariant
    alt?: string
    title?: string
    /** fires on load or error, so a grid can retire its skeleton either way */
    onsettled?: () => void
  } = $props()
</script>

<img
  src={gifRenditionUrl({ did, blobCid, source }, variant)}
  {alt}
  {title}
  loading="lazy"
  decoding="async"
  onload={() => onsettled?.()}
  onerror={() => onsettled?.()}
/>
