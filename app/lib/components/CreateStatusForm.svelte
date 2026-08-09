<script lang="ts">
  import { page } from '$app/stores'
  import { callXrpc } from '$hatk/client'
  import { isCustomEmoji, customEmojiName } from '$lib/utils/emoji'
  import CustomEmoji from './CustomEmoji.svelte'
  import GifImage from './GifImage.svelte'
  import EmojiPicker from './EmojiPicker.svelte'
  import { gifFromRef, type GifRef } from '$lib/utils/gifdex'
  import { toast } from '$lib/toast.svelte'
  import { onMount } from 'svelte'
  import { fly } from 'svelte/transition'
  import { loadPopularEmoji, DEFAULT_FREQUENT } from '$lib/utils/emoji'
  import { buildSuggestions } from '$lib/utils/suggestions'

  let {
    currentEmoji = '😊',
    recent = [],
    oncreated,
  }: { currentEmoji?: string; recent?: string[]; oncreated?: () => void } = $props()

  /** set once you choose for yourself; from then on nothing cycles */
  let picked: string | undefined = $state()
  let popular: string[] = $state([])
  let cycleIndex = $state(0)
  let paused = $state(false)

  let suggestions: string[] = $state([])

  function reshuffle() {
    suggestions = buildSuggestions({ current: currentEmoji, recent, popular })
  }

  // Redraw when the inputs change — popular arrives a moment after mount.
  // Writes only; reading `suggestions` here would make it re-run itself, which
  // is the cycle that froze the emoji picker.
  $effect(() => {
    void currentEmoji
    void recent
    void popular
    reshuffle()
  })
  let suggestion = $derived(suggestions[cycleIndex] ?? suggestions[0] ?? currentEmoji)
  // your choice wins; until then the composer posts whatever it is showing
  let selectedEmoji = $derived(picked ?? suggestion)
  let selectedGif: GifRef | undefined = $state()
  let selectedGifMedia = $derived(gifFromRef(selectedGif))
  let cycling = $derived(!picked && !paused && !selectedGif && suggestions.length > 1)
  let text = $state('')
  let expiresValue = $state('')
  let customDatetime = $state('')
  let showPicker = $state(false)
  let submitting = $state(false)

  const CYCLE_MS = 3200
  let reduced = $state(false)

  onMount(() => {
    reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    loadPopularEmoji(() => callXrpc('dev.hatk.getFeed', { feed: 'popular', limit: 32 }))
      .then((list) => { popular = list })
      .catch(() => { popular = [...DEFAULT_FREQUENT] })

    const id = setInterval(() => {
      if (!cycling) return
      const next = cycleIndex + 1
      if (next >= suggestions.length) {
        // a fresh draw each pass, so it never settles into one fixed order
        cycleIndex = 0
        reshuffle()
      } else {
        cycleIndex = next
      }
    }, CYCLE_MS)
    return () => clearInterval(id)
  })

  function toLocalDatetimeString(date: Date) {
    const offset = date.getTimezoneOffset()
    const local = new Date(date.getTime() - offset * 60 * 1000)
    return local.toISOString().slice(0, 16)
  }

  function onExpiresChange() {
    if (expiresValue === 'custom') {
      const defaultTime = new Date(Date.now() + 60 * 60 * 1000)
      customDatetime = toLocalDatetimeString(defaultTime)
    }
  }

  async function submit(e: Event) {
    e.preventDefault()
    if (!selectedEmoji || !$page.data.viewer) return

    submitting = true
    try {
      const record: {
        $type: string
        emoji: string
        createdAt: string
        text?: string
        expires?: string
        gif?: GifRef
      } = {
        $type: 'io.zzstoatzz.status.record',
        emoji: selectedEmoji,
        createdAt: new Date().toISOString(),
      }
      if (selectedGif) record.gif = selectedGif
      if (text.trim()) record.text = text.trim()
      if (expiresValue === 'custom' && customDatetime) {
        record.expires = new Date(customDatetime).toISOString()
      } else if (expiresValue && expiresValue !== 'custom') {
        record.expires = new Date(Date.now() + parseInt(expiresValue) * 60 * 1000).toISOString()
      }

      await callXrpc('dev.hatk.createRecord', {
        collection: 'io.zzstoatzz.status.record',
        repo: $page.data.viewer.did,
        record,
      })

      text = ''
      expiresValue = ''
      picked = undefined
      selectedGif = undefined
      oncreated?.()
      toast.success('status posted')
    } catch (err: any) {
      toast.error(`could not post: ${err?.message ?? err}`)
    } finally {
      submitting = false
    }
  }
</script>

<form class="status-form" onsubmit={submit}>
  <div class="emoji-input-row">
    <!-- Until you pick, this cycles through your own emoji and then the site's
         popular ones, and posts whatever it is showing. Any contact with it —
         hover, focus, or a finger — stops the cycle where it stands. -->
    <button
      type="button"
      class="emoji-trigger"
      class:cycling
      onclick={() => showPicker = true}
      onpointerenter={() => (paused = true)}
      onpointerleave={() => (paused = false)}
      onpointerdown={() => (paused = true)}
      onfocusin={() => (paused = true)}
      onfocusout={() => (paused = false)}
      aria-label="choose an emoji"
    >
      {#if selectedGifMedia}
        <GifImage
          did={selectedGifMedia.did}
          blobCid={selectedGifMedia.blobCid}
          source={selectedGifMedia.source}
          alt="selected gif"
        />
      {:else}
        {#key selectedEmoji}
          <span
            class="emoji-trigger-glyph"
            in:fly={{ x: reduced ? 0 : 22, duration: reduced ? 0 : 240 }}
            out:fly={{ x: reduced ? 0 : -22, duration: reduced ? 0 : 240 }}
          >
            {#if isCustomEmoji(selectedEmoji)}
              {@const name = customEmojiName(selectedEmoji)}
              <CustomEmoji {name} />
            {:else}
              {selectedEmoji}
            {/if}
          </span>
        {/key}
      {/if}
    </button>
    <input type="text" placeholder="what's happening?" maxlength="256" bind:value={text} />
  </div>
  <div class="form-actions">
    <select bind:value={expiresValue} onchange={onExpiresChange}>
      <option value="">don't clear</option>
      <option value="30">30 min</option>
      <option value="60">1 hour</option>
      <option value="120">2 hours</option>
      <option value="240">4 hours</option>
      <option value="480">8 hours</option>
      <option value="1440">1 day</option>
      <option value="10080">1 week</option>
      <option value="custom">custom...</option>
    </select>
    {#if expiresValue === 'custom'}
      <input type="datetime-local" class="custom-datetime" bind:value={customDatetime} min={toLocalDatetimeString(new Date())} />
    {/if}
    <button type="submit" disabled={submitting}>
      {submitting ? 'setting...' : 'set status'}
    </button>
  </div>
</form>

{#if showPicker}
  <EmojiPicker
    onselect={(emoji, gif) => { picked = emoji; selectedGif = gif; showPicker = false }}
    onclose={() => showPicker = false}
  />
{/if}
