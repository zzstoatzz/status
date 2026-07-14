<script lang="ts">
  import { preferences, savePreferences, ACCENT_COLORS, FONTS, type Preferences } from '$lib/preferences'
  import { get } from 'svelte/store'
  import { onMount } from 'svelte'
  import { Check, ExternalLink, X } from 'lucide-svelte'

  let { onclose }: { onclose: () => void } = $props()

  let current: Preferences = $state({ ...get(preferences) })
  let saving = $state(false)
  let saveError = $state('')
  let closeButton: HTMLButtonElement

  const selectedFont = $derived(FONTS.find((font) => font.value === current.font) ?? FONTS[1])

  function selectColor(color: string) {
    current.accentColor = color
  }

  async function save() {
    saving = true
    saveError = ''
    try {
      await savePreferences(current)
      onclose()
    } catch {
      saveError = 'settings could not be saved. try again.'
    } finally {
      saving = false
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') onclose()
  }

  onMount(() => closeButton.focus())
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="settings-overlay">
  <button class="settings-backdrop" aria-label="close settings" onclick={onclose}></button>
  <div class="settings-modal" role="dialog" aria-modal="true" aria-labelledby="settings-title">
    <div class="settings-header">
      <div>
        <h2 id="settings-title">settings</h2>
        <p>make status feel like yours</p>
      </div>
      <button bind:this={closeButton} class="settings-close" onclick={onclose} aria-label="close settings" title="close settings">
        <X size={18} />
      </button>
    </div>

    <form onsubmit={(event) => { event.preventDefault(); save() }}>
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
              onclick={() => selectColor(color)}
              aria-label="accent color {index + 1}"
              aria-pressed={current.accentColor === color}
            >
              {#if current.accentColor === color}<Check size={14} />{/if}
            </button>
          {/each}
          <label class="custom-color-label" title="custom accent color">
            <span>+</span>
            <input aria-label="custom accent color" type="color" bind:value={current.accentColor} />
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
              onclick={() => current.font = f.value}
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
              onclick={() => current.theme = theme}
            >{theme}</button>
          {/each}
        </div>
      </fieldset>

      {#if saveError}<p class="settings-error" role="alert">{saveError}</p>{/if}

      <div class="settings-footer">
        <a href="https://tangled.org/zzstoatzz.io/status" target="_blank" rel="noopener">
          source on tangled <ExternalLink size={13} />
        </a>
        <button class="save-btn" type="submit" disabled={saving}>
          {saving ? 'saving…' : 'save changes'}
        </button>
      </div>
    </form>
  </div>
</div>
