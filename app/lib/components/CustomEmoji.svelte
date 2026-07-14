<script lang="ts">
  import { bufoImageCandidates } from '$lib/utils/emoji'

  let { name, loading }: { name: string; loading?: 'lazy' | 'eager' } = $props()

  let candidates = $derived(bufoImageCandidates(name))
  let index = $state(0)
  let failed = $state(false)

  $effect(() => {
    name
    index = 0
    failed = false
  })

  function nextCandidate() {
    if (index < candidates.length - 1) {
      index += 1
    } else {
      failed = true
    }
  }
</script>

<svelte:head>
  <link rel="preconnect" href="https://all-the.bufo.zone" crossorigin="anonymous" />
</svelte:head>

{#if failed}
  <span class="custom-emoji-missing" title={name}></span>
{:else}
  <img
    src={candidates[index]}
    alt=""
    title={name}
    {loading}
    decoding="async"
    width="64"
    height="64"
    onerror={nextCandidate}
  />
{/if}
