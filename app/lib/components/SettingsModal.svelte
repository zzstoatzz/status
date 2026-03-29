<script lang="ts">
  import { preferences, savePreferences, ACCENT_COLORS, FONTS, type Preferences } from '$lib/preferences'
  import { get } from 'svelte/store'

  let { onclose }: { onclose: () => void } = $props()

  let current: Preferences = $state({ ...get(preferences) })
  let saving = $state(false)

  function selectColor(color: string) {
    current.accentColor = color
  }

  async function save() {
    saving = true
    try {
      await savePreferences(current)
      onclose()
    } finally {
      saving = false
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="settings-overlay" onclick={(e) => { if (e.target === e.currentTarget) onclose() }}>
  <div class="settings-modal">
    <div class="settings-header">
      <h3>settings</h3>
      <button class="settings-close" onclick={onclose}>&#x2715;</button>
    </div>
    <div class="settings-content">
      <div class="setting-group">
        <label>accent color</label>
        <div class="color-picker">
          {#each ACCENT_COLORS as color}
            <button
              class="color-btn"
              class:active={current.accentColor === color}
              style="background: {color}"
              onclick={() => selectColor(color)}
            ></button>
          {/each}
          <input type="color" class="custom-color-input" bind:value={current.accentColor} />
        </div>
      </div>
      <div class="setting-group">
        <label>font</label>
        <select bind:value={current.font}>
          {#each FONTS as f}
            <option value={f.value}>{f.label}</option>
          {/each}
        </select>
      </div>
      <div class="setting-group">
        <label>theme</label>
        <select bind:value={current.theme}>
          <option value="dark">dark</option>
          <option value="light">light</option>
          <option value="system">system</option>
        </select>
      </div>
    </div>
    <div class="settings-footer">
      <button class="save-btn" onclick={save} disabled={saving}>
        {saving ? 'saving...' : 'save'}
      </button>
    </div>
  </div>
</div>
