<script lang="ts">
  import { page } from '$app/stores'
  import { callXrpc } from '$hatk/client'
  import { isCustomEmoji, customEmojiName } from '$lib/utils/emoji'
  import CustomEmoji from './CustomEmoji.svelte'
  import GifImage from './GifImage.svelte'
  import EmojiPicker from './EmojiPicker.svelte'
  import { gifFromRef, type GifRef } from '$lib/utils/gifdex'

  let { currentEmoji = '😊', oncreated }: { currentEmoji?: string; oncreated?: () => void } = $props()

  // follows the prop until the user picks something, then holds that choice
  let selectedEmoji = $derived(currentEmoji)
  let selectedGif: GifRef | undefined = $state()
  let selectedGifMedia = $derived(gifFromRef(selectedGif))
  let text = $state('')
  let expiresValue = $state('')
  let customDatetime = $state('')
  let showPicker = $state(false)
  let submitting = $state(false)

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
      oncreated?.()
    } catch (err: any) {
      alert('Failed to set status: ' + (err?.message ?? err))
    } finally {
      submitting = false
    }
  }
</script>

<form class="status-form" onsubmit={submit}>
  <div class="emoji-input-row">
    <button type="button" class="emoji-trigger" onclick={() => showPicker = true}>
      {#if selectedGifMedia}
        <GifImage
          did={selectedGifMedia.did}
          blobCid={selectedGifMedia.blobCid}
          animated
          alt="selected gif"
        />
      {:else if isCustomEmoji(selectedEmoji)}
        {@const name = customEmojiName(selectedEmoji)}
        <CustomEmoji {name} />
      {:else}
        {selectedEmoji}
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
    onselect={(emoji, gif) => { selectedEmoji = emoji; selectedGif = gif; showPicker = false }}
    onclose={() => showPicker = false}
  />
{/if}
