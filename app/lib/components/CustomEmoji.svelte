<script lang="ts">
  import { bufoImageUrl } from '$lib/utils/emoji'

  let { name, loading, onsettled }: {
    name: string
    loading?: 'lazy' | 'eager'
    /** fires on load or error — lets a grid retire its skeleton either way */
    onsettled?: () => void
  } = $props()

  let src = $derived(bufoImageUrl(name))
</script>

<svelte:head>
  <link rel="preconnect" href="https://find-bufo.com" crossorigin="anonymous" />
  <link rel="preconnect" href="https://all-the.bufo.zone" crossorigin="anonymous" />
</svelte:head>

<img
  {src}
  alt=""
  title={name}
  {loading}
  decoding="async"
  width="64"
  height="64"
  onload={() => onsettled?.()}
  onerror={() => onsettled?.()}
/>
