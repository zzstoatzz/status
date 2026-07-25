<script lang="ts">
  import { preferences, applyPreferences, savePreferences, ACCENT_COLORS, FONTS, type Preferences } from '$lib/preferences'
  import { AT_CLIENTS, atprotoClient, setPreferredClient } from '$lib/atclients'
  import { get } from 'svelte/store'
  import { onMount } from 'svelte'
  import { Check, X } from 'lucide-svelte'

  let { onclose }: { onclose: () => void } = $props()

  let current: Preferences = $state({ ...get(preferences) })
  let saveError = $state('')
  let closeButton: HTMLButtonElement
  let saveTimer: ReturnType<typeof setTimeout> | undefined

  const selectedFont = $derived(FONTS.find((font) => font.value === current.font) ?? FONTS[1])

  // apply straight away so the change is visible, persist on a debounce so dragging
  // the colour picker doesn't fire a write per frame.
  function update(patch: Partial<Preferences>) {
    current = { ...current, ...patch }
    applyPreferences(current)
    saveError = ''
    clearTimeout(saveTimer)
    saveTimer = setTimeout(flush, 400)
  }

  async function flush() {
    clearTimeout(saveTimer)
    try {
      await savePreferences(current)
    } catch {
      saveError = 'settings could not be saved'
    }
  }

  function close() {
    // don't drop an edit made inside the debounce window
    if (saveTimer) flush()
    onclose()
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') close()
  }

  onMount(() => closeButton.focus())
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="settings-overlay">
  <button class="settings-backdrop" aria-label="close settings" onclick={close}></button>
  <div class="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
    <div class="settings-header">
      <div>
        <h2 id="settings-title">settings</h2>
        <p>make status feel like yours</p>
      </div>
      <button bind:this={closeButton} class="settings-close" onclick={close} aria-label="close settings" title="close settings">
        <X size={18} />
      </button>
    </div>

    <div class="settings-body">
      <div class="settings-preview" style:--preview-accent={current.accentColor} style:font-family={selectedFont.css}>
        <span class="settings-preview-dot"></span>
        <span>shipping something small</span>
        <span class="settings-preview-meta">now</span>
      </div>

      <fieldset class="setting-group">
        <legend>accent</legend>
        <div class="color-picker">
          {#each ACCENT_COLORS as color, index}
            <button
              type="button"
              class="color-btn"
              class:active={current.accentColor === color}
              style="background: {color}"
              onclick={() => update({ accentColor: color })}
              aria-label="accent color {index + 1}"
              aria-pressed={current.accentColor === color}
            >
              {#if current.accentColor === color}<Check size={14} />{/if}
            </button>
          {/each}
          <label class="custom-color-label" title="custom accent color">
            <span>+</span>
            <input aria-label="custom accent color" type="color" value={current.accentColor} oninput={(e) => update({ accentColor: e.currentTarget.value })} />
          </label>
        </div>
      </fieldset>

      <fieldset class="setting-group">
        <legend>type</legend>
        <div class="option-grid font-options">
          {#each FONTS as f}
            <button
              type="button"
              class:active={current.font === f.value}
              aria-pressed={current.font === f.value}
              style:font-family={f.css}
              onclick={() => update({ font: f.value })}
            >{f.label}</button>
          {/each}
        </div>
      </fieldset>

      <fieldset class="setting-group">
        <legend>appearance</legend>
        <div class="option-grid theme-options">
          {#each ['dark', 'light', 'system'] as theme}
            <button
              type="button"
              class:active={current.theme === theme}
              aria-pressed={current.theme === theme}
              onclick={() => update({ theme })}
            >{theme}</button>
          {/each}
        </div>
      </fieldset>

      <fieldset class="setting-group">
        <legend>open profiles in</legend>
        <div class="option-grid client-options">
          {#each AT_CLIENTS as client}
            <button
              type="button"
              class:active={$atprotoClient === client.value}
              aria-pressed={$atprotoClient === client.value}
              onclick={() => setPreferredClient(client.value)}
            >
              <img src={client.iconUrl} alt="" width="18" height="18" loading="lazy" />
              <span>{client.label}</span>
            </button>
          {/each}
        </div>
      </fieldset>

      {#if saveError}<p class="settings-error" role="alert">{saveError}</p>{/if}
    </div>
  </div>
</div>
