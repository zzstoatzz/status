// settings management system
const DEFAULT_SETTINGS = {
    theme: 'system', // system, light, dark, custom
    feedPreference: 'global', // global, following
    
    // typography
    font: {
        family: 'system', // system, mono, serif, sans, custom
        customFamily: '', // for custom font input
        size: 'medium', // small, medium, large, xlarge, custom
        customSize: '16',
        weight: 'normal', // light, normal, medium, bold, black
        lineHeight: 'normal', // tight, normal, relaxed, loose
        letterSpacing: 'normal' // tight, normal, wide
    },
    
    // color scheme
    colors: {
        // main accent color
        accent: '#1DA1F2',
        useSystemAccent: true,
        
        // backgrounds
        bgPrimary: '',
        bgSecondary: '',
        bgTertiary: '',
        
        // text colors
        textPrimary: '',
        textSecondary: '',
        textTertiary: '',
        
        // borders
        borderColor: '',
        borderWidth: 'thin', // none, thin, medium, thick
        
        // gradients
        useGradients: false,
        gradientStart: '',
        gradientEnd: '',
        gradientAngle: '135',
        
        // shadows
        useShadows: true,
        shadowIntensity: 'medium' // none, subtle, medium, strong
    },
    
    // layout & spacing
    layout: {
        maxWidth: 'medium', // narrow, medium, wide, full
        padding: 'normal', // compact, normal, spacious
        borderRadius: 'medium', // none, small, medium, large, round
        statusAlignment: 'left', // left, center, right
        compactMode: false, // reduces whitespace
        cardStyle: 'default', // default, flat, elevated, outlined, glassmorphism
    },
    
    // status display
    status: {
        emojiSize: 'large', // small, medium, large, xlarge, huge
        textSize: 'medium', // small, medium, large, xlarge
        showTimestamp: true,
        timestampFormat: 'relative', // relative, absolute, both
        showUsername: true,
        statusPosition: 'left', // left, center, right
        animateChanges: true,
        glowEffect: false, // add glow to status emoji
        pulseAnimation: false // pulse animation on new status
    },
    
    // emoji customization
    emoji: {
        frequentCount: 12, // 6, 8, 10, 12, 16, 20
        showFrequentBar: true,
        stickyPicker: false,
        pickerPosition: 'bottom', // bottom, top, auto
        pickerSize: 'medium', // small, medium, large
        showRecent: true,
        recentCount: 20,
        defaultEmoji: '😊', // default emoji when none selected
        animateEmojis: true
    },
    
    // feed customization
    feed: {
        cardSpacing: 'normal', // tight, normal, loose
        showAvatars: true,
        avatarSize: 'medium', // small, medium, large
        avatarShape: 'circle', // circle, square, rounded
        showHandles: true,
        showTimestamps: true,
        hoverEffects: true,
        animatedEntrance: true,
        columns: '1', // 1, 2, 3 (for wider screens)
        compactCards: false
    },
    
    // effects & animations
    effects: {
        blur: false, // backdrop blur effects
        blurIntensity: 'medium', // light, medium, heavy
        parallax: false, // parallax scrolling effects
        particles: false, // background particles
        particleCount: 'medium', // few, medium, many
        cssFilters: {
            hue: 0, // -180 to 180
            saturate: 100, // 0 to 200
            brightness: 100, // 0 to 200
            contrast: 100, // 0 to 200
            sepia: 0, // 0 to 100
            invert: 0 // 0 to 100
        }
    },
    
    // background customization
    background: {
        type: 'solid', // solid, gradient, pattern, image, animated
        solidColor: '',
        gradient: {
            type: 'linear', // linear, radial, conic
            colors: [],
            angle: '135'
        },
        pattern: 'none', // dots, lines, grid, waves, custom
        patternOpacity: 0.1,
        image: {
            url: '',
            position: 'center', // positions
            size: 'cover', // cover, contain, auto
            repeat: false,
            fixed: false, // parallax effect
            opacity: 1
        },
        animated: {
            type: 'none', // gradient-shift, floating-shapes, particles
            speed: 'normal' // slow, normal, fast
        }
    },
    
    // accessibility
    accessibility: {
        reduceMotion: false,
        highContrast: false,
        largeText: false,
        focusIndicators: true,
        screenReaderMode: false,
        keyboardNav: true
    },
    
    // advanced
    advanced: {
        customCSS: '', // user's custom CSS
        customHTML: '', // custom HTML for header/footer
        developerMode: false, // shows additional info
        performanceMode: false, // reduces animations
        experimentalFeatures: false
    }
};

class SettingsManager {
    constructor() {
        this.settings = this.loadSettings();
        this.listeners = new Set();
        this.temporaryOverrides = new Map(); // for momentary UI changes
    }

    loadSettings() {
        try {
            const stored = localStorage.getItem('userSettings');
            if (stored) {
                return { ...DEFAULT_SETTINGS, ...JSON.parse(stored) };
            }
        } catch (e) {
            console.error('Failed to load settings:', e);
        }
        return { ...DEFAULT_SETTINGS };
    }

    saveSettings() {
        try {
            localStorage.setItem('userSettings', JSON.stringify(this.settings));
            this.notifyListeners();
        } catch (e) {
            console.error('Failed to save settings:', e);
        }
    }

    get(path, ignoreOverride = false) {
        // check for temporary override first unless explicitly ignored
        if (!ignoreOverride && this.temporaryOverrides.has(path)) {
            return this.temporaryOverrides.get(path);
        }

        const keys = path.split('.');
        let value = this.settings;
        for (const key of keys) {
            value = value?.[key];
        }
        return value;
    }

    set(path, value, isTemporary = false) {
        if (isTemporary) {
            // temporary override that doesn't persist
            this.temporaryOverrides.set(path, value);
            this.notifyListeners(path, value, true);
            return;
        }

        const keys = path.split('.');
        let obj = this.settings;
        for (let i = 0; i < keys.length - 1; i++) {
            if (!obj[keys[i]]) obj[keys[i]] = {};
            obj = obj[keys[i]];
        }
        obj[keys[keys.length - 1]] = value;
        
        // clear any temporary override for this path
        this.temporaryOverrides.delete(path);
        
        this.saveSettings();
    }

    clearTemporaryOverride(path) {
        if (this.temporaryOverrides.has(path)) {
            this.temporaryOverrides.delete(path);
            const value = this.get(path, true); // get real value
            this.notifyListeners(path, value, false);
        }
    }

    clearAllTemporaryOverrides() {
        this.temporaryOverrides.clear();
        this.notifyListeners();
    }

    reset(path = null) {
        if (path) {
            const keys = path.split('.');
            let defaultValue = DEFAULT_SETTINGS;
            for (const key of keys) {
                defaultValue = defaultValue?.[key];
            }
            this.set(path, defaultValue);
        } else {
            this.settings = { ...DEFAULT_SETTINGS };
            this.temporaryOverrides.clear();
            this.saveSettings();
        }
    }

    addListener(callback) {
        this.listeners.add(callback);
    }

    removeListener(callback) {
        this.listeners.delete(callback);
    }

    notifyListeners(changedPath = null, value = null, isTemporary = false) {
        for (const listener of this.listeners) {
            listener({ path: changedPath, value, isTemporary });
        }
    }

    applySettings() {
        // apply theme
        const theme = this.get('theme');
        this.applyTheme(theme);

        // apply typography
        this.applyTypography();

        // apply colors
        this.applyColors();

        // apply layout
        this.applyLayout();

        // apply status styles
        this.applyStatusStyles();

        // apply feed styles
        this.applyFeedStyles();

        // apply effects
        this.applyEffects();

        // apply background
        this.applyBackground();

        // apply accessibility
        this.applyAccessibility();

        // apply custom CSS
        this.applyCustomCSS();
    }

    applyTheme(theme) {
        document.documentElement.classList.remove('light-theme', 'dark-theme');
        
        if (theme === 'light') {
            document.documentElement.classList.add('light-theme');
        } else if (theme === 'dark') {
            document.documentElement.classList.add('dark-theme');
        } else {
            // system theme - check media query
            if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
                document.documentElement.classList.add('dark-theme');
            } else {
                document.documentElement.classList.add('light-theme');
            }
        }
    }

    applyTypography() {
        const font = this.get('font');
        
        // font family
        const fontMap = {
            'system': '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
            'mono': 'ui-monospace, "Cascadia Code", "Source Code Pro", Menlo, monospace',
            'serif': 'ui-serif, Georgia, Cambria, serif',
            'sans': 'ui-sans-serif, system-ui, sans-serif'
        };
        
        let fontFamily = fontMap[font.family] || fontMap.system;
        if (font.family === 'custom' && font.customFamily) {
            fontFamily = font.customFamily;
        }
        
        // font size
        const sizeMap = {
            'small': '14px',
            'medium': '16px',
            'large': '18px',
            'xlarge': '20px'
        };
        
        let fontSize = sizeMap[font.size] || sizeMap.medium;
        if (font.size === 'custom' && font.customSize) {
            fontSize = font.customSize + 'px';
        }
        
        // font weight
        const weightMap = {
            'light': '300',
            'normal': '400',
            'medium': '500',
            'bold': '700',
            'black': '900'
        };
        
        // line height
        const lineHeightMap = {
            'tight': '1.25',
            'normal': '1.5',
            'relaxed': '1.75',
            'loose': '2'
        };
        
        // letter spacing
        const letterSpacingMap = {
            'tight': '-0.05em',
            'normal': '0',
            'wide': '0.05em'
        };
        
        document.documentElement.style.setProperty('--font-family', fontFamily);
        document.documentElement.style.setProperty('--font-size-base', fontSize);
        document.documentElement.style.setProperty('--font-weight', weightMap[font.weight] || '400');
        document.documentElement.style.setProperty('--line-height', lineHeightMap[font.lineHeight] || '1.5');
        document.documentElement.style.setProperty('--letter-spacing', letterSpacingMap[font.letterSpacing] || '0');
    }

    applyColors() {
        const colors = this.get('colors');
        
        // accent color
        if (!colors.useSystemAccent && colors.accent) {
            document.documentElement.style.setProperty('--accent', colors.accent);
            const rgb = this.hexToRgb(colors.accent);
            if (rgb) {
                document.documentElement.style.setProperty('--accent-light', 
                    `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.1)`);
                document.documentElement.style.setProperty('--accent-dark', 
                    this.darkenColor(colors.accent, 20));
            }
        }
        
        // custom colors
        if (colors.bgPrimary) document.documentElement.style.setProperty('--bg-primary', colors.bgPrimary);
        if (colors.bgSecondary) document.documentElement.style.setProperty('--bg-secondary', colors.bgSecondary);
        if (colors.bgTertiary) document.documentElement.style.setProperty('--bg-tertiary', colors.bgTertiary);
        
        if (colors.textPrimary) document.documentElement.style.setProperty('--text-primary', colors.textPrimary);
        if (colors.textSecondary) document.documentElement.style.setProperty('--text-secondary', colors.textSecondary);
        if (colors.textTertiary) document.documentElement.style.setProperty('--text-tertiary', colors.textTertiary);
        
        if (colors.borderColor) document.documentElement.style.setProperty('--border-color', colors.borderColor);
        
        // border width
        const borderWidthMap = {
            'none': '0',
            'thin': '1px',
            'medium': '2px',
            'thick': '3px'
        };
        document.documentElement.style.setProperty('--border-width', borderWidthMap[colors.borderWidth] || '1px');
        
        // gradients
        if (colors.useGradients && colors.gradientStart && colors.gradientEnd) {
            const gradient = `linear-gradient(${colors.gradientAngle}deg, ${colors.gradientStart}, ${colors.gradientEnd})`;
            document.documentElement.style.setProperty('--gradient', gradient);
        }
        
        // shadows
        const shadowMap = {
            'none': 'none',
            'subtle': '0 1px 3px rgba(0,0,0,0.12), 0 1px 2px rgba(0,0,0,0.24)',
            'medium': '0 3px 6px rgba(0,0,0,0.15), 0 2px 4px rgba(0,0,0,0.12)',
            'strong': '0 10px 20px rgba(0,0,0,0.19), 0 6px 6px rgba(0,0,0,0.23)'
        };
        document.documentElement.style.setProperty('--shadow', colors.useShadows ? shadowMap[colors.shadowIntensity] : 'none');
    }

    applyLayout() {
        const layout = this.get('layout');
        
        // max width
        const maxWidthMap = {
            'narrow': '600px',
            'medium': '800px',
            'wide': '1200px',
            'full': '100%'
        };
        document.documentElement.style.setProperty('--max-width', maxWidthMap[layout.maxWidth] || '800px');
        
        // padding
        const paddingMap = {
            'compact': '0.5rem',
            'normal': '1rem',
            'spacious': '2rem'
        };
        document.documentElement.style.setProperty('--padding', paddingMap[layout.padding] || '1rem');
        
        // border radius
        const radiusMap = {
            'none': '0',
            'small': '4px',
            'medium': '8px',
            'large': '16px',
            'round': '9999px'
        };
        document.documentElement.style.setProperty('--radius', radiusMap[layout.borderRadius] || '8px');
        document.documentElement.style.setProperty('--radius-sm', layout.borderRadius === 'none' ? '0' : '4px');
        
        // card style
        document.documentElement.setAttribute('data-card-style', layout.cardStyle);
        
        // compact mode
        if (layout.compactMode) {
            document.documentElement.classList.add('compact-mode');
        } else {
            document.documentElement.classList.remove('compact-mode');
        }
    }
    
    applyStatusStyles() {
        const status = this.get('status');
        
        // emoji size
        const emojiSizeMap = {
            'small': '2rem',
            'medium': '3rem',
            'large': '3.5rem',
            'xlarge': '4.5rem',
            'huge': '6rem'
        };
        document.documentElement.style.setProperty('--status-emoji-size', emojiSizeMap[status.emojiSize] || '3.5rem');
        
        // text size
        const textSizeMap = {
            'small': '1rem',
            'medium': '1.25rem',
            'large': '1.5rem',
            'xlarge': '2rem'
        };
        document.documentElement.style.setProperty('--status-text-size', textSizeMap[status.textSize] || '1.25rem');
        
        // position
        document.documentElement.style.setProperty('--status-align', status.statusPosition || 'left');
        
        // effects
        if (status.glowEffect) {
            document.documentElement.classList.add('status-glow');
        } else {
            document.documentElement.classList.remove('status-glow');
        }
        
        if (status.pulseAnimation) {
            document.documentElement.classList.add('status-pulse');
        } else {
            document.documentElement.classList.remove('status-pulse');
        }
    }
    
    applyFeedStyles() {
        const feed = this.get('feed');
        
        // card spacing
        const spacingMap = {
            'tight': '0.5rem',
            'normal': '1rem',
            'loose': '2rem'
        };
        document.documentElement.style.setProperty('--feed-spacing', spacingMap[feed.cardSpacing] || '1rem');
        
        // avatar size
        const avatarSizeMap = {
            'small': '32px',
            'medium': '48px',
            'large': '64px'
        };
        document.documentElement.style.setProperty('--avatar-size', avatarSizeMap[feed.avatarSize] || '48px');
        
        // avatar shape
        document.documentElement.style.setProperty('--avatar-radius', 
            feed.avatarShape === 'circle' ? '50%' : 
            feed.avatarShape === 'square' ? '0' : '8px');
        
        // columns
        document.documentElement.style.setProperty('--feed-columns', feed.columns || '1');
        
        // toggles
        document.documentElement.classList.toggle('hide-avatars', !feed.showAvatars);
        document.documentElement.classList.toggle('hide-handles', !feed.showHandles);
        document.documentElement.classList.toggle('hide-timestamps', !feed.showTimestamps);
        document.documentElement.classList.toggle('no-hover-effects', !feed.hoverEffects);
        document.documentElement.classList.toggle('compact-cards', feed.compactCards);
    }
    
    applyEffects() {
        const effects = this.get('effects');
        
        // blur
        if (effects.blur) {
            const blurMap = {
                'light': '5px',
                'medium': '10px',
                'heavy': '20px'
            };
            document.documentElement.style.setProperty('--backdrop-blur', blurMap[effects.blurIntensity] || '10px');
            document.documentElement.classList.add('backdrop-blur');
        } else {
            document.documentElement.classList.remove('backdrop-blur');
        }
        
        // css filters
        const filters = effects.cssFilters;
        const filterString = `
            hue-rotate(${filters.hue}deg)
            saturate(${filters.saturate}%)
            brightness(${filters.brightness}%)
            contrast(${filters.contrast}%)
            sepia(${filters.sepia}%)
            invert(${filters.invert}%)
        `.trim();
        
        if (filterString !== 'hue-rotate(0deg) saturate(100%) brightness(100%) contrast(100%) sepia(0%) invert(0%)') {
            document.documentElement.style.setProperty('--css-filter', filterString);
        } else {
            document.documentElement.style.removeProperty('--css-filter');
        }
        
        // parallax
        document.documentElement.classList.toggle('parallax-enabled', effects.parallax);
    }
    
    applyBackground() {
        const bg = this.get('background');
        let bgStyle = '';
        
        switch (bg.type) {
            case 'solid':
                if (bg.solidColor) {
                    bgStyle = bg.solidColor;
                }
                break;
                
            case 'gradient':
                if (bg.gradient.colors.length >= 2) {
                    const type = bg.gradient.type;
                    const colors = bg.gradient.colors.join(', ');
                    if (type === 'linear') {
                        bgStyle = `linear-gradient(${bg.gradient.angle}deg, ${colors})`;
                    } else if (type === 'radial') {
                        bgStyle = `radial-gradient(circle, ${colors})`;
                    } else if (type === 'conic') {
                        bgStyle = `conic-gradient(from ${bg.gradient.angle}deg, ${colors})`;
                    }
                }
                break;
                
            case 'image':
                if (bg.image.url) {
                    bgStyle = `url('${bg.image.url}')`;
                    document.documentElement.style.setProperty('--bg-position', bg.image.position);
                    document.documentElement.style.setProperty('--bg-size', bg.image.size);
                    document.documentElement.style.setProperty('--bg-repeat', bg.image.repeat ? 'repeat' : 'no-repeat');
                    document.documentElement.style.setProperty('--bg-attachment', bg.image.fixed ? 'fixed' : 'scroll');
                    document.documentElement.style.setProperty('--bg-opacity', bg.image.opacity);
                }
                break;
                
            case 'pattern':
                // patterns would be SVG data URIs
                const patterns = {
                    'dots': 'data:image/svg+xml...',
                    'lines': 'data:image/svg+xml...',
                    'grid': 'data:image/svg+xml...',
                    'waves': 'data:image/svg+xml...'
                };
                if (patterns[bg.pattern]) {
                    bgStyle = `url('${patterns[bg.pattern]}')`;
                    document.documentElement.style.setProperty('--pattern-opacity', bg.patternOpacity);
                }
                break;
        }
        
        if (bgStyle) {
            document.documentElement.style.setProperty('--background', bgStyle);
        }
        
        // animated backgrounds would be handled via CSS classes
        document.documentElement.setAttribute('data-bg-animation', bg.animated.type);
        document.documentElement.setAttribute('data-bg-speed', bg.animated.speed);
    }
    
    applyAccessibility() {
        const a11y = this.get('accessibility');
        
        document.documentElement.classList.toggle('reduce-motion', a11y.reduceMotion);
        document.documentElement.classList.toggle('high-contrast', a11y.highContrast);
        document.documentElement.classList.toggle('large-text', a11y.largeText);
        document.documentElement.classList.toggle('focus-visible', a11y.focusIndicators);
        document.documentElement.classList.toggle('screen-reader', a11y.screenReaderMode);
    }
    
    applyCustomCSS() {
        const customCSS = this.get('advanced.customCSS');
        
        // remove old custom style element if exists
        const oldStyle = document.getElementById('user-custom-css');
        if (oldStyle) {
            oldStyle.remove();
        }
        
        // add new custom CSS if provided
        if (customCSS) {
            const style = document.createElement('style');
            style.id = 'user-custom-css';
            style.textContent = customCSS;
            document.head.appendChild(style);
        }
    }

    hexToRgb(hex) {
        const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
        return result ? {
            r: parseInt(result[1], 16),
            g: parseInt(result[2], 16),
            b: parseInt(result[3], 16)
        } : null;
    }

    darkenColor(hex, percent) {
        const rgb = this.hexToRgb(hex);
        if (!rgb) return hex;
        
        const factor = 1 - (percent / 100);
        const r = Math.round(rgb.r * factor);
        const g = Math.round(rgb.g * factor);
        const b = Math.round(rgb.b * factor);
        
        return `#${((1 << 24) + (r << 16) + (g << 8) + b).toString(16).slice(1)}`;
    }

    // migrate old localStorage items to new settings
    migrateOldSettings() {
        const oldTheme = localStorage.getItem('theme');
        if (oldTheme) {
            this.set('theme', oldTheme);
            localStorage.removeItem('theme');
        }

        const oldFeedPref = localStorage.getItem('followingFilterActive');
        if (oldFeedPref === 'true') {
            this.set('feedPreference', 'following');
            localStorage.removeItem('followingFilterActive');
        }
    }
}

// create global instance
window.settingsManager = new SettingsManager();

// migrate old settings on load
window.settingsManager.migrateOldSettings();

// apply settings on load
window.settingsManager.applySettings();

// listen for system theme changes
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
    if (window.settingsManager.get('theme') === 'system') {
        window.settingsManager.applyTheme('system');
    }
});