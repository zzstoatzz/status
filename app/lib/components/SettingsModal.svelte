<script lang="ts">
  import { preferences, savePreferences, ACCENT_COLORS, FONTS, type Preferences } from '$lib/preferences'
  import { AT_CLIENTS, atprotoClient, setPreferredClient } from '$lib/atclients'
  import { Moon, Sun, Monitor } from 'lucide-svelte'
  import { get } from 'svelte/store'

  const THEMES = [
    { value: 'dark', label: 'dark', icon: Moon },
    { value: 'light', label: 'light', icon: Sun },
    { value: 'system', label: 'system', icon: Monitor },
  ]

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
        <div class="theme-picker">
          {#each THEMES as t}
            {@const Icon = t.icon}
            <button
              class="theme-btn"
              class:active={current.theme === t.value}
              onclick={() => current.theme = t.value}
              title={t.label}
            >
              <Icon size={18} />
              <span>{t.label}</span>
            </button>
          {/each}
        </div>
      </div>
      <div class="setting-group">
        <label>open profiles in</label>
        <div class="client-picker">
          {#each AT_CLIENTS as client}
            <button
              class="client-btn"
              class:active={$atprotoClient === client.value}
              onclick={() => setPreferredClient(client.value)}
              title={client.label}
            >
              <img src={client.iconUrl} alt="" width="18" height="18" loading="lazy" />
              <span>{client.label}</span>
            </button>
          {/each}
        </div>
      </div>
    </div>
    <div class="settings-footer">
      <button class="save-btn" onclick={save} disabled={saving}>
        {saving ? 'saving...' : 'save'}
      </button>
    </div>
  </div>
</div>
