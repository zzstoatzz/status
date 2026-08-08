<script lang="ts">
  import { gifBlobUrl } from '$lib/utils/gifdex'

  let {
    did,
    blobCid,
    alt = '',
    title,
    onsettled,
  }: {
    did: string
    blobCid: string
    alt?: string
    title?: string
    /** fires on load or error, so a grid can retire its skeleton either way */
    onsettled?: () => void
  } = $props()
</script>

<img
  src={gifBlobUrl(did, blobCid)}
  {alt}
  {title}
  loading="lazy"
  decoding="async"
  onload={() => onsettled?.()}
  onerror={() => onsettled?.()}
/>
