// Theme Management Module

// Initialize theme on page load
const initTheme = () => {
    const saved = localStorage.getItem('theme');
    const theme = saved || 'system';
    
    if (theme === 'system') {
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        document.body.setAttribute('data-theme', prefersDark ? 'dark' : 'light');
    } else {
        document.body.setAttribute('data-theme', theme);
    }
};

// Toggle between theme modes
const toggleTheme = () => {
    const saved = localStorage.getItem('theme') || 'system';
    const themes = ['system', 'light', 'dark'];
    const currentIndex = themes.indexOf(saved);
    const next = themes[(currentIndex + 1) % themes.length];
    
    localStorage.setItem('theme', next);
    
    if (next === 'system') {
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        document.body.setAttribute('data-theme', prefersDark ? 'dark' : 'light');
    } else {
        document.body.setAttribute('data-theme', next);
    }
    
    // Show theme indicator
    const indicator = document.getElementById('theme-indicator');
    if (indicator) {
        indicator.textContent = next;
        indicator.classList.add('visible');
        setTimeout(() => {
            indicator.classList.remove('visible');
        }, 1500);
    }
};

// Export for use in other modules
window.ThemeManager = {
    init: initTheme,
    toggle: toggleTheme
};