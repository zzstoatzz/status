// Configuration
const CONFIG = {
  server: 'https://zzstoatzz-quickslice-status.fly.dev',
  clientId: 'client_2mP9AwgVHkg1vaSpcWSsKw',
};

// Base path for routing (empty for root domain, '/subpath' for subdirectory)
const BASE_PATH = '';

let client = null;
let userPreferences = null;

// Default preferences
const DEFAULT_PREFERENCES = {
  accentColor: '#4a9eff',
  font: 'mono',
  theme: 'dark'
};

// Available fonts - use simple keys, map to actual CSS in applyPreferences
const FONTS = [
  { value: 'system', label: 'system' },
  { value: 'mono', label: 'mono' },
  { value: 'serif', label: 'serif' },
  { value: 'comic', label: 'comic' },
];

const FONT_CSS = {
  'system': 'system-ui, -apple-system, sans-serif',
  'mono': 'ui-monospace, SF Mono, Monaco, monospace',
  'serif': 'ui-serif, Georgia, serif',
  'comic': 'Comic Sans MS, Comic Sans, cursive',
};

// Preset accent colors
const ACCENT_COLORS = [
  '#4a9eff', // blue (default)
  '#10b981', // green
  '#f59e0b', // amber
  '#ef4444', // red
  '#8b5cf6', // purple
  '#ec4899', // pink
  '#06b6d4', // cyan
  '#f97316', // orange
];

// Apply preferences to the page
function applyPreferences(prefs) {
  const { accentColor, font, theme } = { ...DEFAULT_PREFERENCES, ...prefs };

  document.documentElement.style.setProperty('--accent', accentColor);
  // Map simple font key to actual CSS font-family
  const fontCSS = FONT_CSS[font] || FONT_CSS['mono'];
  document.documentElement.style.setProperty('--font-family', fontCSS);
  document.documentElement.setAttribute('data-theme', theme);

  localStorage.setItem('theme', theme);
}

// Load preferences from server
async function loadPreferences() {
  if (!client) return DEFAULT_PREFERENCES;

  try {
    const user = client.getUser();
    if (!user) return DEFAULT_PREFERENCES;

    const res = await fetch(`${CONFIG.server}/graphql`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        query: `
          query GetPreferences($did: String!) {
            ioZzstoatzzStatusPreferences(
              where: { did: { eq: $did } }
              first: 1
            ) {
              edges { node { accentColor font theme } }
            }
          }
        `,
        variables: { did: user.did }
      })
    });
    const json = await res.json();
    const edges = json.data?.ioZzstoatzzStatusPreferences?.edges || [];

    if (edges.length > 0) {
      userPreferences = edges[0].node;
      return userPreferences;
    }
    return DEFAULT_PREFERENCES;
  } catch (e) {
    console.error('Failed to load preferences:', e);
    return DEFAULT_PREFERENCES;
  }
}

// Save preferences to server
async function savePreferences(prefs) {
  if (!client) return;

  try {
    const user = client.getUser();
    if (!user) return;

    // First, delete any existing preferences records for this user
    const res = await fetch(`${CONFIG.server}/graphql`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        query: `
          query GetExistingPrefs($did: String!) {
            ioZzstoatzzStatusPreferences(where: { did: { eq: $did } }, first: 50) {
              edges { node { uri } }
            }
          }
        `,
        variables: { did: user.did }
      })
    });
    const json = await res.json();
    const existing = json.data?.ioZzstoatzzStatusPreferences?.edges || [];

    // Delete all existing preference records
    for (const edge of existing) {
      const rkey = edge.node.uri.split('/').pop();
      try {
        await client.mutate(`
          mutation DeletePref($rkey: String!) {
            deleteIoZzstoatzzStatusPreferences(rkey: $rkey) { uri }
          }
        `, { rkey });
      } catch (e) {
        console.warn('Failed to delete old pref:', e);
      }
    }

    // Create new preferences record
    await client.mutate(`
      mutation SavePreferences($input: CreateIoZzstoatzzStatusPreferencesInput!) {
        createIoZzstoatzzStatusPreferences(input: $input) { uri }
      }
    `, {
      input: {
        accentColor: prefs.accentColor,
        font: prefs.font,
        theme: prefs.theme
      }
    });

    userPreferences = prefs;
    applyPreferences(prefs);
  } catch (e) {
    console.error('Failed to save preferences:', e);
    alert('Failed to save preferences: ' + e.message);
  }
}

// Create settings modal
function createSettingsModal() {
  const overlay = document.createElement('div');
  overlay.className = 'settings-overlay hidden';
  overlay.innerHTML = `
    <div class="settings-modal">
      <div class="settings-header">
        <h3>settings</h3>
        <button class="settings-close" aria-label="close">✕</button>
      </div>
      <div class="settings-content">
        <div class="setting-group">
          <label>accent color</label>
          <div class="color-picker">
            ${ACCENT_COLORS.map(c => `
              <button class="color-btn" data-color="${c}" style="background: ${c}" title="${c}"></button>
            `).join('')}
            <input type="color" id="custom-color" class="custom-color-input" title="custom color">
          </div>
        </div>
        <div class="setting-group">
          <label>font</label>
          <select id="font-select">
            ${FONTS.map(f => `<option value="${f.value}">${f.label}</option>`).join('')}
          </select>
        </div>
        <div class="setting-group">
          <label>theme</label>
          <select id="theme-select">
            <option value="dark">dark</option>
            <option value="light">light</option>
            <option value="system">system</option>
          </select>
        </div>
      </div>
      <div class="settings-footer">
        <button id="save-settings" class="save-btn">save</button>
      </div>
    </div>
  `;

  const modal = overlay.querySelector('.settings-modal');
  const closeBtn = overlay.querySelector('.settings-close');
  const colorBtns = overlay.querySelectorAll('.color-btn');
  const customColor = overlay.querySelector('#custom-color');
  const fontSelect = overlay.querySelector('#font-select');
  const themeSelect = overlay.querySelector('#theme-select');
  const saveBtn = overlay.querySelector('#save-settings');

  let currentPrefs = { ...DEFAULT_PREFERENCES };

  function updateColorSelection(color) {
    colorBtns.forEach(btn => btn.classList.toggle('active', btn.dataset.color === color));
    customColor.value = color;
    currentPrefs.accentColor = color;
  }

  function open(prefs) {
    currentPrefs = { ...DEFAULT_PREFERENCES, ...prefs };
    updateColorSelection(currentPrefs.accentColor);
    fontSelect.value = currentPrefs.font;
    themeSelect.value = currentPrefs.theme;
    overlay.classList.remove('hidden');
  }

  function close() {
    overlay.classList.add('hidden');
  }

  overlay.addEventListener('click', e => { if (e.target === overlay) close(); });
  closeBtn.addEventListener('click', close);

  colorBtns.forEach(btn => {
    btn.addEventListener('click', () => updateColorSelection(btn.dataset.color));
  });

  customColor.addEventListener('input', () => {
    updateColorSelection(customColor.value);
  });

  fontSelect.addEventListener('change', () => {
    currentPrefs.font = fontSelect.value;
  });

  themeSelect.addEventListener('change', () => {
    currentPrefs.theme = themeSelect.value;
  });

  saveBtn.addEventListener('click', async () => {
    saveBtn.disabled = true;
    saveBtn.textContent = 'saving...';
    await savePreferences(currentPrefs);
    saveBtn.disabled = false;
    saveBtn.textContent = 'save';
    close();
  });

  document.body.appendChild(overlay);
  return { open, close };
}

// Theme (fallback for non-logged-in users)
function initTheme() {
  const saved = localStorage.getItem('theme') || 'dark';
  document.documentElement.setAttribute('data-theme', saved);
}

function toggleTheme() {
  const current = document.documentElement.getAttribute('data-theme');
  const next = current === 'dark' ? 'light' : 'dark';
  document.documentElement.setAttribute('data-theme', next);
  localStorage.setItem('theme', next);

  // If logged in, also update preferences
  if (userPreferences) {
    userPreferences.theme = next;
    savePreferences(userPreferences);
  }
}

// Timestamp formatting (ported from original status app)
const TimestampFormatter = {
  formatRelative(date, now = new Date()) {
    const diffMs = now - date;
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMs < 30000) return 'just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) {
      const remainingMins = diffMins % 60;
      return remainingMins === 0 ? `${diffHours}h ago` : `${diffHours}h ${remainingMins}m ago`;
    }
    if (diffDays < 7) {
      const remainingHours = diffHours % 24;
      return remainingHours === 0 ? `${diffDays}d ago` : `${diffDays}d ${remainingHours}h ago`;
    }

    const timeStr = date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true }).toLowerCase();
    if (date.getFullYear() === now.getFullYear()) {
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }) + ', ' + timeStr;
    }
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }) + ', ' + timeStr;
  },

  formatCompact(date, now = new Date()) {
    const diffMs = now - date;
    const diffDays = Math.floor(diffMs / 86400000);

    if (date.toDateString() === now.toDateString()) {
      return date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true }).toLowerCase();
    }
    const yesterday = new Date(now);
    yesterday.setDate(yesterday.getDate() - 1);
    if (date.toDateString() === yesterday.toDateString()) {
      return 'yesterday, ' + date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true }).toLowerCase();
    }
    if (diffDays < 7) {
      const dayName = date.toLocaleDateString('en-US', { weekday: 'short' }).toLowerCase();
      const time = date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true }).toLowerCase();
      return `${dayName}, ${time}`;
    }
    if (date.getFullYear() === now.getFullYear()) {
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: 'numeric', minute: '2-digit', hour12: true }).toLowerCase();
    }
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric', hour: 'numeric', minute: '2-digit', hour12: true }).toLowerCase();
  },

  getFullTimestamp(date) {
    const dayName = date.toLocaleDateString('en-US', { weekday: 'long' });
    const monthDay = date.toLocaleDateString('en-US', { month: 'long', day: 'numeric', year: 'numeric' });
    const time = date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', second: '2-digit', hour12: true });
    const tzAbbr = date.toLocaleTimeString('en-US', { timeZoneName: 'short' }).split(' ').pop();
    return `${dayName}, ${monthDay} at ${time} ${tzAbbr}`;
  }
};

function relativeTime(dateStr, format = 'relative') {
  const date = new Date(dateStr);
  return format === 'compact'
    ? TimestampFormatter.formatCompact(date)
    : TimestampFormatter.formatRelative(date);
}

function formatExpiration(dateStr) {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = date - now;

  // Already expired - show how long ago
  if (diffMs <= 0) {
    const agoMs = Math.abs(diffMs);
    const agoMins = Math.floor(agoMs / 60000);
    if (agoMins < 1) return 'expired';
    if (agoMins < 60) return `expired ${agoMins}m ago`;
    const agoHours = Math.floor(agoMs / 3600000);
    if (agoHours < 24) return `expired ${agoHours}h ago`;
    const agoDays = Math.floor(agoMs / 86400000);
    return `expired ${agoDays}d ago`;
  }

  // Future - show when it clears
  return `clears ${relativeTimeFuture(dateStr)}`;
}

function relativeTimeFuture(dateStr) {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = date - now;

  if (diffMs <= 0) return 'now';

  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 1) return 'in less than a minute';
  if (diffMins < 60) return `in ${diffMins}m`;
  if (diffHours < 24) {
    const remainingMins = diffMins % 60;
    return remainingMins === 0 ? `in ${diffHours}h` : `in ${diffHours}h ${remainingMins}m`;
  }
  if (diffDays < 7) {
    const remainingHours = diffHours % 24;
    return remainingHours === 0 ? `in ${diffDays}d` : `in ${diffDays}d ${remainingHours}h`;
  }

  // For longer times, show the date
  const timeStr = date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true }).toLowerCase();
  if (date.getFullYear() === now.getFullYear()) {
    return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' }) + ', ' + timeStr;
  }
  return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }) + ', ' + timeStr;
}

function fullTimestamp(dateStr) {
  return TimestampFormatter.getFullTimestamp(new Date(dateStr));
}

// Emoji picker
let emojiData = null;
let bufoList = null;
let userFrequentEmojis = null;
const DEFAULT_FREQUENT_EMOJIS = ['😊', '👍', '❤️', '😂', '🎉', '🔥', '✨', '💯', '🚀', '💪', '🙏', '👏', '😴', '🤔', '👀', '💻'];

async function loadUserFrequentEmojis() {
  if (userFrequentEmojis) return userFrequentEmojis;
  if (!client) return DEFAULT_FREQUENT_EMOJIS;

  try {
    const user = client.getUser();
    if (!user) return DEFAULT_FREQUENT_EMOJIS;

    // Fetch user's status history to count emoji usage
    const res = await fetch(`${CONFIG.server}/graphql`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        query: `
          query GetUserEmojis($did: String!) {
            ioZzstoatzzStatusRecord(
              first: 100
              where: { did: { eq: $did } }
            ) {
              edges { node { emoji } }
            }
          }
        `,
        variables: { did: user.did }
      })
    });
    const json = await res.json();
    const emojis = json.data?.ioZzstoatzzStatusRecord?.edges?.map(e => e.node.emoji) || [];

    if (emojis.length === 0) return DEFAULT_FREQUENT_EMOJIS;

    // Count emoji frequency
    const counts = {};
    emojis.forEach(e => { counts[e] = (counts[e] || 0) + 1; });

    // Sort by frequency and take top 16
    const sorted = Object.entries(counts)
      .sort((a, b) => b[1] - a[1])
      .slice(0, 16)
      .map(([emoji]) => emoji);

    userFrequentEmojis = sorted.length > 0 ? sorted : DEFAULT_FREQUENT_EMOJIS;
    return userFrequentEmojis;
  } catch (e) {
    console.error('Failed to load frequent emojis:', e);
    return DEFAULT_FREQUENT_EMOJIS;
  }
}

async function loadBufoList() {
  if (bufoList) return bufoList;
  const res = await fetch('/bufos.json');
  if (!res.ok) throw new Error('Failed to load bufos');
  bufoList = await res.json();
  return bufoList;
}

async function loadEmojiData() {
  if (emojiData) return emojiData;
  try {
    const response = await fetch('https://cdn.jsdelivr.net/npm/emoji-datasource@15.1.0/emoji.json');
    if (!response.ok) throw new Error('Failed to fetch');
    const data = await response.json();

    const emojis = {};
    const categories = { frequent: DEFAULT_FREQUENT_EMOJIS, people: [], nature: [], food: [], activity: [], travel: [], objects: [], symbols: [], flags: [] };
    const categoryMap = {
      'Smileys & Emotion': 'people', 'People & Body': 'people', 'Animals & Nature': 'nature',
      'Food & Drink': 'food', 'Activities': 'activity', 'Travel & Places': 'travel',
      'Objects': 'objects', 'Symbols': 'symbols', 'Flags': 'flags'
    };

    data.forEach(emoji => {
      const char = emoji.unified.split('-').map(u => String.fromCodePoint(parseInt(u, 16))).join('');
      const keywords = [...(emoji.short_names || []), ...(emoji.name ? emoji.name.toLowerCase().split(/[\s_-]+/) : [])];
      emojis[char] = keywords;
      const cat = categoryMap[emoji.category];
      if (cat && categories[cat]) categories[cat].push(char);
    });

    emojiData = { emojis, categories };
    return emojiData;
  } catch (e) {
    console.error('Failed to load emoji data:', e);
    return { emojis: {}, categories: { frequent: DEFAULT_FREQUENT_EMOJIS, people: [], nature: [], food: [], activity: [], travel: [], objects: [], symbols: [], flags: [] } };
  }
}

function searchEmojis(query, data) {
  if (!query) return [];
  const q = query.toLowerCase();
  return Object.entries(data.emojis)
    .filter(([char, keywords]) => keywords.some(k => k.includes(q)))
    .map(([char]) => char)
    .slice(0, 50);
}

function createEmojiPicker(onSelect) {
  const overlay = document.createElement('div');
  overlay.className = 'emoji-picker-overlay hidden';
  overlay.innerHTML = `
    <div class="emoji-picker">
      <div class="emoji-picker-header">
        <h3>pick an emoji</h3>
        <button class="emoji-picker-close" aria-label="close">✕</button>
      </div>
      <input type="text" class="emoji-search" placeholder="search emojis...">
      <div class="emoji-categories">
        <button class="category-btn active" data-category="frequent">⭐</button>
        <button class="category-btn" data-category="custom">🐸</button>
        <button class="category-btn" data-category="people">😊</button>
        <button class="category-btn" data-category="nature">🌿</button>
        <button class="category-btn" data-category="food">🍔</button>
        <button class="category-btn" data-category="activity">⚽</button>
        <button class="category-btn" data-category="travel">✈️</button>
        <button class="category-btn" data-category="objects">💡</button>
        <button class="category-btn" data-category="symbols">💕</button>
        <button class="category-btn" data-category="flags">🏁</button>
      </div>
      <div class="emoji-grid"></div>
      <div class="bufo-helper hidden"><a href="https://find-bufo.fly.dev/" target="_blank">need help finding a bufo?</a></div>
    </div>
  `;

  const picker = overlay.querySelector('.emoji-picker');
  const grid = overlay.querySelector('.emoji-grid');
  const search = overlay.querySelector('.emoji-search');
  const closeBtn = overlay.querySelector('.emoji-picker-close');
  const categoryBtns = overlay.querySelectorAll('.category-btn');
  const bufoHelper = overlay.querySelector('.bufo-helper');

  let currentCategory = 'frequent';
  let data = null;

  async function renderCategory(cat) {
    currentCategory = cat;
    categoryBtns.forEach(b => b.classList.toggle('active', b.dataset.category === cat));
    bufoHelper.classList.toggle('hidden', cat !== 'custom');

    if (cat === 'custom') {
      grid.classList.add('bufo-grid');
      grid.innerHTML = '<div class="loading">loading bufos...</div>';
      try {
        const bufos = await loadBufoList();
        grid.innerHTML = bufos.map(name => `
          <button class="emoji-btn bufo-btn" data-emoji="custom:${name}" title="${name}">
            <img src="https://all-the.bufo.zone/${name}.png" alt="${name}" loading="lazy" onerror="this.src='https://all-the.bufo.zone/${name}.gif'">
          </button>
        `).join('');
      } catch (e) {
        grid.innerHTML = '<div class="no-results">failed to load bufos</div>';
      }
      return;
    }

    grid.classList.remove('bufo-grid');

    // Load user's frequent emojis for the frequent category
    if (cat === 'frequent') {
      grid.innerHTML = '<div class="loading">loading...</div>';
      const frequentEmojis = await loadUserFrequentEmojis();
      grid.innerHTML = frequentEmojis.map(e => {
        if (e.startsWith('custom:')) {
          const name = e.replace('custom:', '');
          return `<button class="emoji-btn bufo-btn" data-emoji="${e}" title="${name}">
            <img src="https://all-the.bufo.zone/${name}.png" alt="${name}" onerror="this.src='https://all-the.bufo.zone/${name}.gif'">
          </button>`;
        }
        return `<button class="emoji-btn" data-emoji="${e}">${e}</button>`;
      }).join('');
      return;
    }

    if (!data) data = await loadEmojiData();
    const emojis = data.categories[cat] || [];
    grid.innerHTML = emojis.map(e => `<button class="emoji-btn" data-emoji="${e}">${e}</button>`).join('');
  }

  function close() {
    overlay.classList.add('hidden');
    search.value = '';
  }

  function open() {
    overlay.classList.remove('hidden');
    renderCategory('frequent');
    search.focus();
  }

  overlay.addEventListener('click', e => { if (e.target === overlay) close(); });
  closeBtn.addEventListener('click', close);
  categoryBtns.forEach(btn => btn.addEventListener('click', () => renderCategory(btn.dataset.category)));

  grid.addEventListener('click', e => {
    const btn = e.target.closest('.emoji-btn');
    if (btn) {
      onSelect(btn.dataset.emoji);
      close();
    }
  });

  search.addEventListener('input', async () => {
    const q = search.value.trim();
    if (!q) { renderCategory(currentCategory); return; }

    // Search both emojis and bufos
    if (!data) data = await loadEmojiData();
    const emojiResults = searchEmojis(q, data);

    // Search bufos by name
    let bufoResults = [];
    try {
      const bufos = await loadBufoList();
      const qLower = q.toLowerCase();
      bufoResults = bufos.filter(name => name.toLowerCase().includes(qLower)).slice(0, 30);
    } catch (e) { /* ignore */ }

    grid.classList.remove('bufo-grid');
    bufoHelper.classList.add('hidden');

    if (emojiResults.length === 0 && bufoResults.length === 0) {
      grid.innerHTML = '<div class="no-results">no emojis found</div>';
      return;
    }

    let html = '';
    // Show emoji results first
    html += emojiResults.map(e => `<button class="emoji-btn" data-emoji="${e}">${e}</button>`).join('');
    // Then bufo results
    html += bufoResults.map(name => `
      <button class="emoji-btn bufo-btn" data-emoji="custom:${name}" title="${name}">
        <img src="https://all-the.bufo.zone/${name}.png" alt="${name}" onerror="this.src='https://all-the.bufo.zone/${name}.gif'">
      </button>
    `).join('');

    grid.innerHTML = html;
  });

  document.body.appendChild(overlay);
  return { open, close };
}

// Render emoji (handles custom:name format)
function renderEmoji(emoji) {
  if (emoji && emoji.startsWith('custom:')) {
    const name = emoji.slice(7);
    return `<img src="https://all-the.bufo.zone/${name}.png" alt="${name}" title="${name}" onerror="this.src='https://all-the.bufo.zone/${name}.gif'">`;
  }
  return emoji || '-';
}

function escapeHtml(str) {
  if (!str) return '';
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

// Parse markdown links [text](url) and return HTML
function parseLinks(text) {
  if (!text) return '';
  // First escape HTML, then parse markdown links
  const escaped = escapeHtml(text);
  // Match [text](url) pattern
  return escaped.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (match, linkText, url) => {
    // Validate URL (basic check)
    if (url.startsWith('http://') || url.startsWith('https://')) {
      return `<a href="${url}" target="_blank" rel="noopener">${linkText}</a>`;
    }
    return match;
  });
}

// Resolve handle to DID
async function resolveHandle(handle) {
  const res = await fetch(`https://bsky.social/xrpc/com.atproto.identity.resolveHandle?handle=${encodeURIComponent(handle)}`);
  if (!res.ok) return null;
  const data = await res.json();
  return data.did;
}

// Resolve DID to handle
async function resolveDidToHandle(did) {
  const res = await fetch(`https://plc.directory/${did}`);
  if (!res.ok) return null;
  const data = await res.json();
  // alsoKnownAs is like ["at://handle"]
  if (data.alsoKnownAs && data.alsoKnownAs.length > 0) {
    return data.alsoKnownAs[0].replace('at://', '');
  }
  return null;
}

// Router
function getRoute() {
  const path = window.location.pathname;
  if (path === '/' || path === '/index.html') return { page: 'home' };
  if (path === '/feed' || path === '/feed.html') return { page: 'feed' };
  if (path.startsWith('/@')) {
    const handle = path.slice(2);
    return { page: 'profile', handle };
  }
  return { page: '404' };
}

// Render home page
async function renderHome() {
  const main = document.getElementById('main-content');
  document.getElementById('page-title').textContent = 'status';

  if (typeof QuicksliceClient === 'undefined') {
    main.innerHTML = '<div class="center">failed to load. check console.</div>';
    return;
  }

  try {
    client = await QuicksliceClient.createQuicksliceClient({
      server: CONFIG.server,
      clientId: CONFIG.clientId,
      redirectUri: window.location.origin + '/',
    });
    console.log('Client created with server:', CONFIG.server, 'clientId:', CONFIG.clientId);

    if (window.location.search.includes('code=')) {
      console.log('Got OAuth callback with code, handling...');
      try {
        const result = await client.handleRedirectCallback();
        console.log('handleRedirectCallback result:', result);
      } catch (err) {
        console.error('handleRedirectCallback error:', err);
      }
      window.history.replaceState({}, document.title, '/');
    }

    const isAuthed = await client.isAuthenticated();

    if (!isAuthed) {
      main.innerHTML = `
        <div class="center">
          <p>share your status on the atproto network</p>
          <form id="login-form">
            <input type="text" id="handle-input" placeholder="your.handle" required>
            <button type="submit">log in</button>
          </form>
        </div>
      `;
      document.getElementById('login-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        const handle = document.getElementById('handle-input').value.trim();
        if (handle && client) {
          await client.loginWithRedirect({ handle });
        }
      });
    } else {
      const user = client.getUser();
      if (!user) {
        // Token might be invalid, log out
        await client.logout();
        window.location.reload();
        return;
      }

      // Load statuses first (includes actorHandle to avoid PLC lookup)
      const res = await fetch(`${CONFIG.server}/graphql`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          query: `
            query GetUserStatuses($did: String!) {
              ioZzstoatzzStatusRecord(
                first: 100
                where: { did: { eq: $did } }
                sortBy: [{ field: "createdAt", direction: DESC }]
              ) {
                edges { node { uri did actorHandle emoji text createdAt expires } }
              }
            }
          `,
          variables: { did: user.did }
        })
      });
      const json = await res.json();
      const statuses = json.data.ioZzstoatzzStatusRecord.edges.map(e => e.node);

      // Get handle from statuses if available, otherwise fall back to PLC lookup
      const handle = statuses.length > 0 && statuses[0].actorHandle
        ? statuses[0].actorHandle
        : (await resolveDidToHandle(user.did) || user.did);

      // Load and apply preferences, set up settings/logout buttons
      const prefs = await loadPreferences();
      applyPreferences(prefs);

      // Show settings button and set up modal
      const settingsBtn = document.getElementById('settings-btn');
      settingsBtn.classList.remove('hidden');
      const settingsModal = createSettingsModal();
      settingsBtn.addEventListener('click', () => settingsModal.open(userPreferences || prefs));

      // Add logout button to header nav (if not already there)
      if (!document.getElementById('logout-btn')) {
        const nav = document.querySelector('header nav');
        const logoutBtn = document.createElement('button');
        logoutBtn.id = 'logout-btn';
        logoutBtn.className = 'nav-btn';
        logoutBtn.setAttribute('aria-label', 'log out');
        logoutBtn.setAttribute('title', 'log out');
        logoutBtn.innerHTML = `
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
            <polyline points="16 17 21 12 16 7"></polyline>
            <line x1="21" y1="12" x2="9" y2="12"></line>
          </svg>
        `;
        logoutBtn.addEventListener('click', async () => {
          await client.logout();
          window.location.href = '/';
        });
        nav.appendChild(logoutBtn);
      }

      // Set page title with Bluesky profile link
      document.getElementById('page-title').innerHTML = `<a href="https://bsky.app/profile/${handle}" target="_blank">@${handle}</a>`;

      let currentHtml = '<span class="big-emoji">-</span>';
      let historyHtml = '';

      if (statuses.length > 0) {
        const current = statuses[0];
        const expiresHtml = current.expires ? ` • ${formatExpiration(current.expires)}` : '';
        currentHtml = `
          <span class="big-emoji">${renderEmoji(current.emoji)}</span>
          <div class="status-info">
            ${current.text ? `<span id="current-text">${parseLinks(current.text)}</span>` : ''}
            <span class="meta">since ${relativeTime(current.createdAt)}${expiresHtml}</span>
          </div>
        `;
        if (statuses.length > 1) {
          historyHtml = '<section class="history"><h2>history</h2><div id="history-list">';
          statuses.slice(1).forEach(s => {
            // Extract rkey from URI (at://did/collection/rkey)
            const rkey = s.uri.split('/').pop();
            historyHtml += `
              <div class="status-item">
                <span class="emoji">${renderEmoji(s.emoji)}</span>
                <div class="content">
                  <div>${s.text ? `<span class="text">${parseLinks(s.text)}</span>` : ''}</div>
                  <span class="time">${relativeTime(s.createdAt)}</span>
                </div>
                <button class="delete-btn" data-rkey="${escapeHtml(rkey)}" title="delete">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <line x1="18" y1="6" x2="6" y2="18"></line>
                    <line x1="6" y1="6" x2="18" y2="18"></line>
                  </svg>
                </button>
              </div>
            `;
          });
          historyHtml += '</div></section>';
        }
      }

      const currentEmoji = statuses.length > 0 ? statuses[0].emoji : '😊';

      main.innerHTML = `
        <div class="profile-card">
          <div class="current-status">${currentHtml}</div>
        </div>
        <form id="status-form" class="status-form">
          <div class="emoji-input-row">
            <button type="button" id="emoji-trigger" class="emoji-trigger">
              <span id="selected-emoji">${renderEmoji(currentEmoji)}</span>
            </button>
            <input type="hidden" id="emoji-input" value="${escapeHtml(currentEmoji)}">
            <input type="text" id="text-input" placeholder="what's happening?" maxlength="256">
          </div>
          <div class="form-actions">
            <select id="expires-select">
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
            <input type="datetime-local" id="custom-datetime" class="custom-datetime hidden">
            <button type="submit">set status</button>
          </div>
        </form>
        ${historyHtml}
      `;

      // Set up emoji picker
      const emojiInput = document.getElementById('emoji-input');
      const selectedEmojiEl = document.getElementById('selected-emoji');
      const emojiPicker = createEmojiPicker((emoji) => {
        emojiInput.value = emoji;
        selectedEmojiEl.innerHTML = renderEmoji(emoji);
      });
      document.getElementById('emoji-trigger').addEventListener('click', () => emojiPicker.open());

      // Custom datetime toggle
      const expiresSelect = document.getElementById('expires-select');
      const customDatetime = document.getElementById('custom-datetime');

      // Helper to format date for datetime-local input (local timezone)
      function toLocalDatetimeString(date) {
        const offset = date.getTimezoneOffset();
        const local = new Date(date.getTime() - offset * 60 * 1000);
        return local.toISOString().slice(0, 16);
      }

      expiresSelect.addEventListener('change', () => {
        if (expiresSelect.value === 'custom') {
          customDatetime.classList.remove('hidden');
          // Set min to now (prevent past dates)
          const now = new Date();
          customDatetime.min = toLocalDatetimeString(now);
          // Default to 1 hour from now
          const defaultTime = new Date(Date.now() + 60 * 60 * 1000);
          customDatetime.value = toLocalDatetimeString(defaultTime);
        } else {
          customDatetime.classList.add('hidden');
        }
      });

      document.getElementById('status-form').addEventListener('submit', async (e) => {
        e.preventDefault();
        const emoji = document.getElementById('emoji-input').value.trim();
        const text = document.getElementById('text-input').value.trim();
        const expiresVal = document.getElementById('expires-select').value;
        const customDt = document.getElementById('custom-datetime').value;

        if (!emoji) return;

        const input = { emoji, createdAt: new Date().toISOString() };
        if (text) input.text = text;
        if (expiresVal === 'custom' && customDt) {
          input.expires = new Date(customDt).toISOString();
        } else if (expiresVal && expiresVal !== 'custom') {
          input.expires = new Date(Date.now() + parseInt(expiresVal) * 60 * 1000).toISOString();
        }

        try {
          await client.mutate(`
            mutation CreateStatus($input: CreateIoZzstoatzzStatusRecordInput!) {
              createIoZzstoatzzStatusRecord(input: $input) { uri }
            }
          `, { input });
          window.location.reload();
        } catch (err) {
          console.error('Failed to create status:', err);
          alert('Failed to set status: ' + err.message);
        }
      });

      // Delete buttons
      document.querySelectorAll('.delete-btn').forEach(btn => {
        btn.addEventListener('click', async () => {
          const rkey = btn.dataset.rkey;
          if (!confirm('Delete this status?')) return;

          try {
            await client.mutate(`
              mutation DeleteStatus($rkey: String!) {
                deleteIoZzstoatzzStatusRecord(rkey: $rkey) { uri }
              }
            `, { rkey });
            window.location.reload();
          } catch (err) {
            console.error('Failed to delete status:', err);
            alert('Failed to delete: ' + err.message);
          }
        });
      });
    }
  } catch (e) {
    console.error('Failed to init:', e);
    main.innerHTML = '<div class="center">failed to initialize. check console.</div>';
  }
}

// Render feed page
let feedCursor = null;
let feedHasMore = true;

async function renderFeed(append = false) {
  const main = document.getElementById('main-content');
  document.getElementById('page-title').textContent = 'global feed';

  if (!append) {
    // Initialize auth UI for header elements
    await initAuthUI();
    main.innerHTML = '<div id="feed-list" class="feed-list"><div class="center">loading...</div></div><div id="load-more" class="center hidden"><button id="load-more-btn">load more</button></div><div id="end-of-feed" class="center hidden"><span class="meta">you\'ve reached the end</span></div>';
  }

  const feedList = document.getElementById('feed-list');

  try {
    const res = await fetch(`${CONFIG.server}/graphql`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        query: `
          query GetFeed($after: String) {
            ioZzstoatzzStatusRecord(first: 20, after: $after, sortBy: [{ field: "createdAt", direction: DESC }]) {
              edges { node { uri did actorHandle emoji text createdAt } cursor }
              pageInfo { hasNextPage endCursor }
            }
          }
        `,
        variables: { after: append ? feedCursor : null }
      })
    });

    const json = await res.json();
    const data = json.data.ioZzstoatzzStatusRecord;
    const statuses = data.edges.map(e => e.node);
    feedCursor = data.pageInfo.endCursor;
    feedHasMore = data.pageInfo.hasNextPage;

    if (!append) {
      feedList.innerHTML = '';
    }

    statuses.forEach((status) => {
      const handle = status.actorHandle || status.did.slice(8, 28);
      const div = document.createElement('div');
      div.className = 'status-item';
      div.innerHTML = `
        <span class="emoji">${renderEmoji(status.emoji)}</span>
        <div class="content">
          <div>
            <a href="/@${handle}" class="author">@${handle}</a>
            ${status.text ? `<span class="text">${parseLinks(status.text)}</span>` : ''}
          </div>
          <span class="time">${relativeTime(status.createdAt)}</span>
        </div>
      `;
      feedList.appendChild(div);
    });

    const loadMore = document.getElementById('load-more');
    const endOfFeed = document.getElementById('end-of-feed');
    if (feedHasMore) {
      loadMore.classList.remove('hidden');
      endOfFeed.classList.add('hidden');
    } else {
      loadMore.classList.add('hidden');
      endOfFeed.classList.remove('hidden');
    }

    // Attach load more handler
    const btn = document.getElementById('load-more-btn');
    if (btn && !btn.dataset.bound) {
      btn.dataset.bound = 'true';
      btn.addEventListener('click', () => renderFeed(true));
    }
  } catch (e) {
    console.error('Failed to load feed:', e);
    if (!append) {
      feedList.innerHTML = '<div class="center">failed to load feed</div>';
    }
  }
}

// Render profile page
async function renderProfile(handle) {
  const main = document.getElementById('main-content');
  const pageTitle = document.getElementById('page-title');

  // Initialize auth UI for header elements
  await initAuthUI();

  pageTitle.innerHTML = `<a href="https://bsky.app/profile/${handle}" target="_blank">@${handle}</a>`;

  main.innerHTML = '<div class="center">loading...</div>';

  try {
    // Resolve handle to DID
    const did = await resolveHandle(handle);
    if (!did) {
      main.innerHTML = '<div class="center">user not found</div>';
      return;
    }

    const res = await fetch(`${CONFIG.server}/graphql`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        query: `
          query GetUserStatuses($did: String!) {
            ioZzstoatzzStatusRecord(first: 20, where: { did: { eq: $did } }, sortBy: [{ field: "createdAt", direction: DESC }]) {
              edges { node { uri did emoji text createdAt expires } }
            }
          }
        `,
        variables: { did }
      })
    });

    const json = await res.json();
    const statuses = json.data.ioZzstoatzzStatusRecord.edges.map(e => e.node);

    if (statuses.length === 0) {
      main.innerHTML = '<div class="center">no statuses yet</div>';
      return;
    }

    const current = statuses[0];
    const expiresHtml = current.expires ? ` • ${formatExpiration(current.expires)}` : '';
    let html = `
      <div class="profile-card">
        <div class="current-status">
          <span class="big-emoji">${renderEmoji(current.emoji)}</span>
          <div class="status-info">
            ${current.text ? `<span id="current-text">${parseLinks(current.text)}</span>` : ''}
            <span class="meta">${relativeTime(current.createdAt)}${expiresHtml}</span>
          </div>
        </div>
      </div>
    `;

    if (statuses.length > 1) {
      html += '<section class="history"><h2>history</h2><div class="feed-list">';
      statuses.slice(1).forEach(status => {
        html += `
          <div class="status-item">
            <span class="emoji">${renderEmoji(status.emoji)}</span>
            <div class="content">
              <div>${status.text ? `<span class="text">${parseLinks(status.text)}</span>` : ''}</div>
              <span class="time">${relativeTime(status.createdAt)}</span>
            </div>
          </div>
        `;
      });
      html += '</div></section>';
    }

    main.innerHTML = html;
  } catch (e) {
    console.error('Failed to load profile:', e);
    main.innerHTML = '<div class="center">failed to load profile</div>';
  }
}

// Update nav active state - hide current page icon, show the other
function updateNavActive(page) {
  const navHome = document.getElementById('nav-home');
  const navFeed = document.getElementById('nav-feed');
  // Hide the nav icon for the current page, show the other
  if (navHome) navHome.classList.toggle('hidden', page === 'home');
  if (navFeed) navFeed.classList.toggle('hidden', page === 'feed');
}

// Initialize auth state for header (settings, logout) - used by all pages
async function initAuthUI() {
  if (typeof QuicksliceClient === 'undefined') return;

  try {
    client = await QuicksliceClient.createQuicksliceClient({
      server: CONFIG.server,
      clientId: CONFIG.clientId,
      redirectUri: window.location.origin + '/',
    });

    const isAuthed = await client.isAuthenticated();
    if (!isAuthed) return;

    const user = client.getUser();
    if (!user) return;

    // Load and apply preferences
    const prefs = await loadPreferences();
    applyPreferences(prefs);

    // Show settings button and set up modal
    const settingsBtn = document.getElementById('settings-btn');
    settingsBtn.classList.remove('hidden');
    const settingsModal = createSettingsModal();
    settingsBtn.addEventListener('click', () => settingsModal.open(userPreferences || prefs));

    // Add logout button to header nav (if not already there)
    if (!document.getElementById('logout-btn')) {
      const nav = document.querySelector('header nav');
      const logoutBtn = document.createElement('button');
      logoutBtn.id = 'logout-btn';
      logoutBtn.className = 'nav-btn';
      logoutBtn.setAttribute('aria-label', 'log out');
      logoutBtn.setAttribute('title', 'log out');
      logoutBtn.innerHTML = `
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
          <polyline points="16 17 21 12 16 7"></polyline>
          <line x1="21" y1="12" x2="9" y2="12"></line>
        </svg>
      `;
      logoutBtn.addEventListener('click', async () => {
        await client.logout();
        window.location.href = '/';
      });
      nav.appendChild(logoutBtn);
    }

    return { user, prefs };
  } catch (e) {
    console.error('Failed to init auth UI:', e);
    return null;
  }
}

// Init
document.addEventListener('DOMContentLoaded', () => {
  initTheme();

  const themeBtn = document.getElementById('theme-toggle');
  if (themeBtn) {
    themeBtn.addEventListener('click', toggleTheme);
  }

  const route = getRoute();
  updateNavActive(route.page);

  if (route.page === 'home') {
    renderHome();
  } else if (route.page === 'feed') {
    renderFeed();
  } else if (route.page === 'profile') {
    renderProfile(route.handle);
  } else {
    document.getElementById('main-content').innerHTML = '<div class="center">page not found</div>';
  }
});
