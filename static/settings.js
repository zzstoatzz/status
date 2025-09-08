// Shared font map configuration
const FONT_MAP = {
    'system': '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
    'mono': '"JetBrains Mono", "Fira Code", "Cascadia Code", monospace',
    'serif': 'ui-serif, Georgia, Cambria, serif',
    'comic': '"Comic Sans MS", "Comic Sans", cursive'
};

// Check if user is authenticated by looking for auth-specific data
function isAuthenticated() {
    // Check for data attribute that indicates authentication status
    return document.body.dataset.authenticated === 'true';
}

// Helper to save preferences to API
async function savePreferencesToAPI(updates) {
    if (!isAuthenticated()) return;
    
    try {
        await fetch('/api/preferences', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(updates)
        });
    } catch (err) {
        console.log('Failed to save preferences to server');
    }
}

// Apply font to the document
function applyFont(fontKey) {
    const fontFamily = FONT_MAP[fontKey] || FONT_MAP.mono;
    document.documentElement.style.setProperty('--font-family', fontFamily);
}

// Apply accent color to the document
function applyAccentColor(color) {
    document.documentElement.style.setProperty('--accent', color);
}