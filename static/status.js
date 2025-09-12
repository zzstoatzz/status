// Status Page Functionality

// Initialize status page settings
const initStatusSettings = async () => {
    // Try to load from API first, fall back to localStorage
    let savedFont = localStorage.getItem('fontFamily') || 'mono';
    let savedAccent = localStorage.getItem('accentColor') || '#1DA1F2';
    
    // If user is the owner, fetch from API
    const isOwner = document.querySelector('.settings-toggle');
    if (isOwner) {
        try {
            const response = await fetch('/api/preferences');
            if (response.ok) {
                const data = await response.json();
                if (!data.error) {
                    savedFont = data.font_family || savedFont;
                    savedAccent = data.accent_color || savedAccent;
                    // Sync to localStorage
                    localStorage.setItem('fontFamily', savedFont);
                    localStorage.setItem('accentColor', savedAccent);
                }
            }
        } catch (err) {
            console.log('Using localStorage preferences');
        }
    }
    
    // Apply font family using shared FONT_MAP from settings.js
    if (typeof FONT_MAP !== 'undefined') {
        document.documentElement.style.setProperty('--font-family', FONT_MAP[savedFont] || FONT_MAP.mono);
    }
    
    // Update buttons
    document.querySelectorAll('.font-btn').forEach(btn => {
        btn.classList.toggle('active', btn.dataset.font === savedFont);
    });
    
    // Apply accent color
    document.documentElement.style.setProperty('--accent', savedAccent);
    const accentInput = document.getElementById('accent-color');
    if (accentInput) {
        accentInput.value = savedAccent;
    }
};

// Initialize emoji picker functionality
const initEmojiPicker = () => {
    // Get DOM elements
    const emojiButton = document.getElementById('emoji-button');
    const emojiPopup = document.getElementById('emoji-popup');
    const emojiGrid = document.getElementById('emoji-grid');
    const customEmojiGrid = document.getElementById('custom-emoji-grid');
    const statusInput = document.getElementById('status-input');
    
    if (!emojiButton || !emojiPopup) {
        console.log('Emoji picker elements not found');
        return;
    }
    
    // Toggle emoji popup
    emojiButton.addEventListener('click', (e) => {
        e.preventDefault();
        const isVisible = emojiPopup.style.display === 'block';
        emojiPopup.style.display = isVisible ? 'none' : 'block';
        
        // Populate grids if showing
        if (!isVisible) {
            populateEmojiGrids();
        }
    });
    
    // Close popup when clicking outside
    document.addEventListener('click', (e) => {
        if (!emojiButton.contains(e.target) && !emojiPopup.contains(e.target)) {
            emojiPopup.style.display = 'none';
        }
    });
    
    // Handle emoji selection
    const handleEmojiClick = (emoji, isCustom = false) => {
        if (statusInput) {
            if (isCustom) {
                statusInput.value = `custom:${emoji}`;
            } else {
                statusInput.value = emoji;
            }
            
            // Update preview if it exists
            const preview = document.querySelector('.status-preview .status-emoji');
            if (preview) {
                if (isCustom) {
                    preview.innerHTML = `<img src="/emojis/${emoji}.png" alt="${emoji}" title="${emoji}" class="custom-emoji-display" onerror="this.onerror=null; this.src='/emojis/${emoji}.gif';">`;
                } else {
                    preview.textContent = emoji;
                }
            }
        }
        emojiPopup.style.display = 'none';
    };
    
    // Populate emoji grids
    const populateEmojiGrids = () => {
        // Standard emojis
        if (emojiGrid && window.emojiCategories) {
            emojiGrid.innerHTML = '';
            Object.entries(window.emojiCategories).forEach(([category, emojis]) => {
                const categorySection = document.createElement('div');
                categorySection.className = 'emoji-category';
                
                const categoryTitle = document.createElement('h4');
                categoryTitle.textContent = category;
                categoryTitle.className = 'emoji-category-title';
                categorySection.appendChild(categoryTitle);
                
                const categoryGrid = document.createElement('div');
                categoryGrid.className = 'emoji-category-grid';
                
                emojis.forEach(emoji => {
                    const emojiSpan = document.createElement('span');
                    emojiSpan.className = 'emoji-item';
                    emojiSpan.textContent = emoji;
                    emojiSpan.title = emoji;
                    emojiSpan.addEventListener('click', () => handleEmojiClick(emoji));
                    categoryGrid.appendChild(emojiSpan);
                });
                
                categorySection.appendChild(categoryGrid);
                emojiGrid.appendChild(categorySection);
            });
        }
        
        // Custom emojis
        if (customEmojiGrid && window.emojiCategories && window.emojiCategories.custom) {
            customEmojiGrid.innerHTML = '';
            
            if (window.emojiCategories.custom.length > 0) {
                window.emojiCategories.custom.forEach(emojiName => {
                    const emojiImg = document.createElement('img');
                    emojiImg.className = 'custom-emoji-item';
                    emojiImg.src = `/emojis/${emojiName}.png`;
                    emojiImg.alt = emojiName;
                    emojiImg.title = emojiName;
                    emojiImg.onerror = function() { 
                        this.onerror = null; 
                        this.src = `/emojis/${emojiName}.gif`; 
                    };
                    emojiImg.addEventListener('click', () => handleEmojiClick(emojiName, true));
                    customEmojiGrid.appendChild(emojiImg);
                });
            } else {
                const emptyMessage = document.createElement('div');
                emptyMessage.className = 'empty-custom-emojis';
                emptyMessage.textContent = 'No custom emojis yet';
                customEmojiGrid.appendChild(emptyMessage);
            }
        }
    };
    
    // Listen for custom emoji uploads to refresh the picker
    document.addEventListener('custom-emoji-uploaded', () => {
        // Refresh custom emoji data and repopulate
        fetch('/api/custom-emojis').then(response => response.json())
            .then(data => {
                if (window.emojiCategories) {
                    window.emojiCategories.custom = data.emojis || [];
                    populateEmojiGrids();
                }
            })
            .catch(err => console.log('Failed to refresh custom emojis'));
    });
};

// Initialize status page functionality
const initStatusPage = async () => {
    // Initialize settings
    await initStatusSettings();
    
    // Initialize emoji picker
    initEmojiPicker();
    
    // Settings toggle
    const settingsToggle = document.getElementById('settings-toggle');
    const settingsPanel = document.getElementById('simple-settings');
    if (settingsToggle && settingsPanel) {
        settingsToggle.addEventListener('click', () => {
            settingsPanel.classList.toggle('hidden');
        });
    }
    
    // Helper to save preferences to API
    const savePreferencesToAPI = async (updates) => {
        if (!document.querySelector('.settings-toggle')) return;
        
        try {
            await fetch('/api/preferences', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(updates)
            });
        } catch (err) {
            console.log('Failed to save preferences to server');
        }
    };
    
    // Font family buttons
    document.querySelectorAll('.font-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            const font = btn.dataset.font;
            localStorage.setItem('fontFamily', font);
            
            // Update UI
            document.querySelectorAll('.font-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            // Apply using shared FONT_MAP from settings.js
            if (typeof FONT_MAP !== 'undefined') {
                document.documentElement.style.setProperty('--font-family', FONT_MAP[font] || FONT_MAP.mono);
            }
            
            // Save to API if owner
            if (document.querySelector('.settings-toggle')) {
                savePreferencesToAPI({ font_family: font });
            }
        });
    });
    
    // Accent color
    const accentInput = document.getElementById('accent-color');
    if (accentInput) {
        accentInput.addEventListener('input', () => {
            const color = accentInput.value;
            localStorage.setItem('accentColor', color);
            document.documentElement.style.setProperty('--accent', color);
            
            // Save to API if owner
            if (document.querySelector('.settings-toggle')) {
                savePreferencesToAPI({ accent_color: color });
            }
        });
    }
    
    // Color presets
    document.querySelectorAll('.color-preset').forEach(btn => {
        btn.addEventListener('click', () => {
            const color = btn.dataset.color;
            localStorage.setItem('accentColor', color);
            document.documentElement.style.setProperty('--accent', color);
            if (accentInput) {
                accentInput.value = color;
            }
            
            // Save to API if owner
            if (document.querySelector('.settings-toggle')) {
                savePreferencesToAPI({ accent_color: color });
            }
        });
    });
    
    // Webhook configuration button
    const webhookConfigButton = document.getElementById('open-webhook-config');
    if (webhookConfigButton) {
        webhookConfigButton.addEventListener('click', (e) => {
            e.preventDefault();
            // Look for existing webhook modal functionality
            const webhookModal = document.getElementById('webhook-modal');
            if (webhookModal) {
                webhookModal.style.display = 'block';
            } else if (typeof openWebhookConfig === 'function') {
                openWebhookConfig();
            } else {
                // Fallback: navigate to webhook configuration page or show modal
                console.log('Opening webhook configuration...');
                // This might need to open a specific URL or modal depending on the app structure
                window.location.href = '#webhook-config';
            }
        });
    }
};

// Export for global use
window.StatusManager = {
    init: initStatusPage
};