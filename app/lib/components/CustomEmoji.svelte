<script lang="ts">
  import { bufoImageUrl } from '$lib/utils/emoji'

  let { name, loading }: { name: string; loading?: 'lazy' | 'eager' } = $props()

  let src = $derived(bufoImageUrl(name))
</script>

<svelte:head>
  <link rel="preconnect" href="https://find-bufo.com" crossorigin="anonymous" />
  <link rel="preconnect" href="https://all-the.bufo.zone" crossorigin="anonymous" />
</svelte:head>

<img {src} alt="" title={name} {loading} decoding="async" width="64" height="64" />
