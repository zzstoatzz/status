// settings UI controller
document.addEventListener('DOMContentLoaded', () => {
    const sm = window.settingsManager;
    if (!sm) {
        console.error('Settings manager not loaded');
        return;
    }

    // navigation
    const navItems = document.querySelectorAll('.nav-item[data-section]');
    const sections = document.querySelectorAll('.settings-section');

    navItems.forEach(item => {
        item.addEventListener('click', () => {
            const section = item.dataset.section;
            
            // update nav
            navItems.forEach(n => n.classList.remove('active'));
            item.classList.add('active');
            
            // update sections
            sections.forEach(s => s.classList.remove('active'));
            document.getElementById(section)?.classList.add('active');
        });
    });

    // option buttons
    document.querySelectorAll('.option-btn').forEach(btn => {
        const setting = btn.dataset.setting;
        const value = btn.dataset.value;
        
        // set initial state
        if (setting && sm.get(setting) == value) {
            btn.classList.add('active');
        }
        
        btn.addEventListener('click', () => {
            // update UI
            btn.parentElement.querySelectorAll('.option-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            
            // update setting
            if (setting) {
                sm.set(setting, value);
                sm.applySettings();
                updatePreview();
            }
        });
    });

    // checkboxes
    const checkboxMappings = {
        'use-system-accent': 'colors.useSystemAccent',
        'compact-mode': 'layout.compactMode',
        'show-timestamp': 'status.showTimestamp',
        'show-username': 'status.showUsername',
        'animate-changes': 'status.animateChanges',
        'glow-effect': 'status.glowEffect',
        'pulse-animation': 'status.pulseAnimation',
        'show-avatars': 'feed.showAvatars',
        'show-handles': 'feed.showHandles',
        'show-feed-timestamps': 'feed.showTimestamps',
        'hover-effects': 'feed.hoverEffects',
        'animated-entrance': 'feed.animatedEntrance',
        'compact-cards': 'feed.compactCards',
        'show-frequent-bar': 'emoji.showFrequentBar',
        'sticky-picker': 'emoji.stickyPicker',
        'show-recent': 'emoji.showRecent',
        'animate-emojis': 'emoji.animateEmojis',
        'enable-blur': 'effects.blur',
        'enable-parallax': 'effects.parallax',
        'enable-particles': 'effects.particles',
        'reduce-motion': 'accessibility.reduceMotion',
        'high-contrast': 'accessibility.highContrast',
        'large-text': 'accessibility.largeText',
        'focus-indicators': 'accessibility.focusIndicators',
        'screen-reader-mode': 'accessibility.screenReaderMode',
        'keyboard-nav': 'accessibility.keyboardNav',
        'developer-mode': 'advanced.developerMode',
        'performance-mode': 'advanced.performanceMode',
        'experimental-features': 'advanced.experimentalFeatures'
    };

    Object.entries(checkboxMappings).forEach(([id, setting]) => {
        const checkbox = document.getElementById(id);
        if (checkbox) {
            // set initial state
            checkbox.checked = sm.get(setting);
            
            checkbox.addEventListener('change', () => {
                sm.set(setting, checkbox.checked);
                sm.applySettings();
                updatePreview();
            });
        }
    });

    // color pickers
    const accentColor = document.getElementById('accent-color');
    if (accentColor) {
        accentColor.value = sm.get('colors.accent');
        accentColor.addEventListener('input', () => {
            sm.set('colors.accent', accentColor.value);
            sm.set('colors.useSystemAccent', false);
            document.getElementById('use-system-accent').checked = false;
            sm.applySettings();
            updatePreview();
        });
    }

    // preset colors
    document.querySelectorAll('.color-preset').forEach(btn => {
        btn.addEventListener('click', () => {
            const color = btn.dataset.color;
            accentColor.value = color;
            sm.set('colors.accent', color);
            sm.set('colors.useSystemAccent', false);
            document.getElementById('use-system-accent').checked = false;
            sm.applySettings();
            updatePreview();
        });
    });

    // sliders
    const sliders = {
        'filter-hue': 'effects.cssFilters.hue',
        'filter-saturate': 'effects.cssFilters.saturate',
        'filter-brightness': 'effects.cssFilters.brightness',
        'filter-contrast': 'effects.cssFilters.contrast'
    };

    Object.entries(sliders).forEach(([id, setting]) => {
        const slider = document.getElementById(id);
        if (slider) {
            slider.value = sm.get(setting);
            const valueDisplay = slider.parentElement.querySelector('.slider-value');
            
            const updateValue = () => {
                const value = slider.value;
                sm.set(setting, parseInt(value));
                
                if (id === 'filter-hue') {
                    valueDisplay.textContent = `${value}°`;
                } else {
                    valueDisplay.textContent = `${value}%`;
                }
                
                sm.applySettings();
                updatePreview();
            };
            
            slider.addEventListener('input', updateValue);
            updateValue();
        }
    });

    // text inputs
    const customFont = document.getElementById('custom-font');
    if (customFont) {
        customFont.value = sm.get('font.customFamily');
        customFont.addEventListener('input', () => {
            sm.set('font.customFamily', customFont.value);
            if (customFont.value) {
                sm.set('font.family', 'custom');
            }
            sm.applySettings();
            updatePreview();
        });
    }

    const defaultEmoji = document.getElementById('default-emoji');
    if (defaultEmoji) {
        defaultEmoji.value = sm.get('emoji.defaultEmoji');
        defaultEmoji.addEventListener('input', () => {
            sm.set('emoji.defaultEmoji', defaultEmoji.value);
            updatePreview();
        });
    }

    // custom CSS
    const customCSS = document.getElementById('custom-css');
    if (customCSS) {
        customCSS.value = sm.get('advanced.customCSS');
        customCSS.addEventListener('input', () => {
            sm.set('advanced.customCSS', customCSS.value);
            sm.applySettings();
            updatePreview();
        });
    }

    // background type switcher
    const bgTypeButtons = document.querySelectorAll('[data-setting="background.type"]');
    bgTypeButtons.forEach(btn => {
        btn.addEventListener('click', () => {
            const type = btn.dataset.value;
            
            // hide all bg option groups
            document.getElementById('bg-solid-options').style.display = 'none';
            document.getElementById('bg-gradient-options').style.display = 'none';
            document.getElementById('bg-pattern-options').style.display = 'none';
            document.getElementById('bg-image-options').style.display = 'none';
            
            // show relevant one
            const optionsId = `bg-${type}-options`;
            const optionsEl = document.getElementById(optionsId);
            if (optionsEl) {
                optionsEl.style.display = 'block';
            }
        });
    });

    // background color
    const bgSolidColor = document.getElementById('bg-solid-color');
    if (bgSolidColor) {
        bgSolidColor.value = sm.get('background.solidColor') || '#ffffff';
        bgSolidColor.addEventListener('input', () => {
            sm.set('background.solidColor', bgSolidColor.value);
            sm.applySettings();
            updatePreview();
        });
    }

    // gradient colors
    const updateGradientColors = () => {
        const colors = Array.from(document.querySelectorAll('.gradient-color')).map(input => input.value);
        sm.set('background.gradient.colors', colors);
        sm.applySettings();
        updatePreview();
    };

    document.querySelectorAll('.gradient-color').forEach(input => {
        input.addEventListener('input', updateGradientColors);
    });

    document.querySelector('.add-gradient-color')?.addEventListener('click', () => {
        const container = document.querySelector('.gradient-colors');
        const newInput = document.createElement('input');
        newInput.type = 'color';
        newInput.className = 'gradient-color';
        newInput.value = '#000000';
        newInput.addEventListener('input', updateGradientColors);
        container.insertBefore(newInput, container.lastElementChild);
    });

    // background image
    const bgImageUrl = document.getElementById('bg-image-url');
    if (bgImageUrl) {
        bgImageUrl.value = sm.get('background.image.url');
        bgImageUrl.addEventListener('input', () => {
            sm.set('background.image.url', bgImageUrl.value);
            sm.applySettings();
            updatePreview();
        });
    }

    // presets
    const presets = {
        minimal: {
            theme: 'light',
            'colors.shadowIntensity': 'none',
            'layout.cardStyle': 'flat',
            'layout.borderRadius': 'small',
            'font.family': 'sans',
            'effects.blur': false,
            'status.animateChanges': false
        },
        vibrant: {
            theme: 'light',
            'colors.accent': '#FF6B6B',
            'colors.useSystemAccent': false,
            'colors.shadowIntensity': 'medium',
            'layout.cardStyle': 'elevated',
            'layout.borderRadius': 'large',
            'status.glowEffect': true,
            'feed.animatedEntrance': true
        },
        midnight: {
            theme: 'dark',
            'colors.accent': '#45B7D1',
            'colors.useSystemAccent': false,
            'colors.shadowIntensity': 'strong',
            'layout.cardStyle': 'elevated',
            'effects.blur': true,
            'effects.blurIntensity': 'medium'
        },
        retro: {
            theme: 'dark',
            'font.family': 'mono',
            'colors.accent': '#00FF00',
            'colors.useSystemAccent': false,
            'layout.borderRadius': 'none',
            'layout.cardStyle': 'outlined',
            'colors.shadowIntensity': 'none'
        },
        glassmorphism: {
            theme: 'light',
            'layout.cardStyle': 'glassmorphism',
            'effects.blur': true,
            'effects.blurIntensity': 'heavy',
            'colors.shadowIntensity': 'subtle',
            'layout.borderRadius': 'large'
        },
        brutalist: {
            theme: 'light',
            'font.family': 'sans',
            'font.weight': 'black',
            'layout.borderRadius': 'none',
            'colors.shadowIntensity': 'strong',
            'colors.accent': '#000000',
            'colors.useSystemAccent': false
        },
        pastel: {
            theme: 'light',
            'colors.accent': '#DDA0DD',
            'colors.useSystemAccent': false,
            'layout.borderRadius': 'round',
            'colors.shadowIntensity': 'subtle',
            'font.family': 'serif'
        },
        cyberpunk: {
            theme: 'dark',
            'colors.accent': '#FF00FF',
            'colors.useSystemAccent': false,
            'status.glowEffect': true,
            'effects.blur': true,
            'font.family': 'mono',
            'layout.cardStyle': 'outlined'
        }
    };

    document.querySelectorAll('.preset-card').forEach(card => {
        card.addEventListener('click', () => {
            const presetName = card.dataset.preset;
            const preset = presets[presetName];
            
            if (preset) {
                Object.entries(preset).forEach(([key, value]) => {
                    sm.set(key, value);
                });
                sm.applySettings();
                updateUI();
                updatePreview();
                
                // show feedback
                card.style.background = 'var(--accent)';
                setTimeout(() => {
                    card.style.background = '';
                }, 500);
            }
        });
    });

    // save button
    document.getElementById('save-settings')?.addEventListener('click', () => {
        sm.saveSettings();
        const btn = document.getElementById('save-settings');
        btn.textContent = 'saved!';
        btn.classList.add('saved');
        setTimeout(() => {
            btn.textContent = 'save changes';
            btn.classList.remove('saved');
        }, 2000);
    });

    // reset all
    document.getElementById('reset-all')?.addEventListener('click', () => {
        if (confirm('reset all settings to defaults?')) {
            sm.reset();
            sm.applySettings();
            updateUI();
            updatePreview();
        }
    });

    // export settings
    document.getElementById('export-settings')?.addEventListener('click', () => {
        const settings = JSON.stringify(sm.settings, null, 2);
        const blob = new Blob([settings], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'status-sphere-settings.json';
        a.click();
        URL.revokeObjectURL(url);
    });

    // import settings
    document.getElementById('import-settings')?.addEventListener('change', (e) => {
        const file = e.target.files[0];
        if (file) {
            const reader = new FileReader();
            reader.onload = (event) => {
                try {
                    const settings = JSON.parse(event.target.result);
                    Object.entries(settings).forEach(([key, value]) => {
                        sm.settings[key] = value;
                    });
                    sm.saveSettings();
                    sm.applySettings();
                    updateUI();
                    updatePreview();
                    alert('settings imported successfully!');
                } catch (err) {
                    alert('failed to import settings: invalid file');
                }
            };
            reader.readAsText(file);
        }
    });

    // preview toggle
    document.getElementById('toggle-preview')?.addEventListener('click', () => {
        const panel = document.querySelector('.preview-panel');
        const btn = document.getElementById('toggle-preview');
        
        if (panel.style.display === 'none') {
            panel.style.display = 'block';
            btn.textContent = 'hide preview';
        } else {
            panel.style.display = 'none';
            btn.textContent = 'show preview';
        }
    });

    // update UI to match current settings
    function updateUI() {
        // update all option buttons
        document.querySelectorAll('.option-btn').forEach(btn => {
            const setting = btn.dataset.setting;
            const value = btn.dataset.value;
            if (setting && sm.get(setting) == value) {
                btn.parentElement.querySelectorAll('.option-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
            }
        });
        
        // update checkboxes
        Object.entries(checkboxMappings).forEach(([id, setting]) => {
            const checkbox = document.getElementById(id);
            if (checkbox) {
                checkbox.checked = sm.get(setting);
            }
        });
        
        // update other inputs
        if (accentColor) accentColor.value = sm.get('colors.accent');
        if (customFont) customFont.value = sm.get('font.customFamily');
        if (defaultEmoji) defaultEmoji.value = sm.get('emoji.defaultEmoji');
        if (customCSS) customCSS.value = sm.get('advanced.customCSS');
    }

    // update preview iframe
    function updatePreview() {
        const iframe = document.getElementById('preview-iframe');
        if (iframe && iframe.contentWindow) {
            // send message to iframe to update settings
            iframe.contentWindow.postMessage({
                type: 'updateSettings',
                settings: sm.settings
            }, '*');
        }
    }

    // listen for settings changes from other tabs
    window.addEventListener('storage', (e) => {
        if (e.key === 'userSettings') {
            sm.settings = JSON.parse(e.newValue);
            sm.applySettings();
            updateUI();
        }
    });

    // initial UI update
    updateUI();
});