// Feed Page Functionality
// IIFE to encapsulate module variables and prevent global namespace pollution
(() => {
    // Module-scoped variables
    let isLoading = false;
    let offset = 0;
    let hasMore = true;
    let followingDids = null;
    let filterActive = false;
    let currentUserDid = null;

// Initialize feed settings
const initFeedSettings = async () => {
    // Try to load from API first, fall back to localStorage
    let savedFont = localStorage.getItem('fontFamily') || 'mono';
    let savedAccent = localStorage.getItem('accentColor') || '#1DA1F2';
    
    // If user is logged in, fetch from API
    const isLoggedIn = document.querySelector('.settings-toggle');
    if (isLoggedIn) {
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

// Fetch user's following list
const fetchFollowing = async () => {
    try {
        const response = await fetch('/api/following');
        if (!response.ok) {
            console.error('Failed to fetch following list');
            return null;
        }
        const data = await response.json();
        return data.follows;
    } catch (error) {
        console.error('Error fetching following:', error);
        return null;
    }
};

// Check if we need to load more content after filtering
const checkNeedMoreContent = () => {
    // Check if filtered content fills the viewport
    setTimeout(() => {
        if (document.documentElement.scrollHeight <= window.innerHeight && hasMore && !isLoading) {
            loadMoreStatuses();
        }
    }, 50); // Small delay to ensure layout is updated
};

// Apply following filter to existing statuses
const applyFollowingFilter = (active) => {
    const statusItems = document.querySelectorAll('.status-item');
    
    statusItems.forEach(item => {
        if (!active || !followingDids) {
            // Show all if filter is off or we don't have following data
            item.style.display = '';
        } else {
            const authorDid = item.getAttribute('data-did');
            // Check if this author is in our following list OR is the current user
            if (followingDids.includes(authorDid) || authorDid === currentUserDid) {
                item.style.display = '';
            } else {
                item.style.display = 'none';
            }
        }
    });
    
    // After filtering, check if we need more content
    if (active) {
        checkNeedMoreContent();
    }
};

// Variables already declared at module scope above

// Load more statuses
const loadMoreStatuses = async () => {
    if (isLoading || !hasMore) return;
    
    isLoading = true;
    const loadingIndicator = document.getElementById('loading-indicator');
    loadingIndicator.style.display = 'block';
    
    try {
        const response = await fetch(`/api/feed?offset=${offset}&limit=20`);
        const data = await response.json();
        const newStatuses = Array.isArray(data) ? data : (data.statuses || []);
        
        if (newStatuses.length === 0) {
            hasMore = false;
            loadingIndicator.style.display = 'none';
            document.getElementById('end-of-feed').style.display = 'block';
            return;
        }
        
        const statusList = document.querySelector('.status-list');
        
        // Render new statuses
        newStatuses.forEach(status => {
            const statusItem = document.createElement('div');
            statusItem.className = 'status-item';
            statusItem.setAttribute('data-did', status.author_did);
            
            let emojiHtml = '';
            if (status.status.startsWith('custom:')) {
                const emojiName = status.status.substring(7);
                emojiHtml = `<img src="/emojis/${emojiName}.png" alt="${emojiName}" title="${emojiName}" class="custom-emoji-display" onerror="this.onerror=null; this.src='/emojis/${emojiName}.gif';">`;
            } else {
                emojiHtml = `<span title="${status.status}">${status.status}</span>`;
            }
            
            // Build expiry HTML if present
            let expiryHtml = '';
            if (status.expires_at) {
                const expiryDate = new Date(status.expires_at);
                const now = new Date();
                if (expiryDate > now) {
                    expiryHtml = ` • <span class="local-time" data-timestamp="${status.expires_at}" data-prefix="expires"></span>`;
                } else {
                    expiryHtml = ' • expired';
                }
            }
            
            const displayName = status.handle || status.author_did;
            const profileUrl = status.handle ? `/@${status.handle}` : '#';
            
            statusItem.innerHTML = `
                <span class="status-emoji">${emojiHtml}</span>
                <div class="status-content">
                    <div class="status-main">
                        <a class="status-author" href="${profileUrl}">@${displayName}</a>
                        ${status.text ? `<span class="status-text">${status.text}</span>` : ''}
                    </div>
                    <div class="status-meta">
                        <span class="local-time" data-timestamp="${status.started_at}" data-format="relative"></span>${expiryHtml}
                    </div>
                </div>
            `;
            
            statusList.appendChild(statusItem);
            // Render markdown links in the newly added item
            if (window.renderMarkdownLinksIn) {
                window.renderMarkdownLinksIn(statusItem);
            }
        });
        
        // Re-initialize timestamps for newly added elements
        if (typeof TimestampFormatter !== 'undefined') {
            TimestampFormatter.initialize();
        }
        
        // Apply filter to newly added items if active
        if (filterActive && followingDids) {
            applyFollowingFilter(true);
        }
        
        offset += newStatuses.length;
        if (!Array.isArray(data) && typeof data === 'object') {
            if (typeof data.next_offset === 'number') offset = data.next_offset;
            if (typeof data.has_more === 'boolean') hasMore = data.has_more;
        }
        loadingIndicator.style.display = 'none';
    } catch (error) {
        console.error('Error loading more statuses:', error);
        loadingIndicator.style.display = 'none';
    } finally {
        isLoading = false;
    }
};

// Check scroll position for infinite scrolling
const checkScroll = () => {
    const scrollHeight = document.documentElement.scrollHeight;
    const scrollTop = window.scrollY;
    const clientHeight = window.innerHeight;
    
    // Load more when user is 200px from the bottom
    if (scrollTop + clientHeight >= scrollHeight - 200) {
        loadMoreStatuses();
    }
};

// Initialize feed page
const initFeedPage = async (initialOffset = 0, userDid = null) => {
    offset = initialOffset;
    currentUserDid = userDid;
    
    await initFeedSettings();
    
    // Always load initial page of statuses so feed is never empty on first render
    try { 
        await loadMoreStatuses(); 
    } catch (e) { 
        console.error('initial load failed', e); 
    }
    
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
            
            // Save to API if logged in
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
            
            // Save to API if logged in
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
            
            // Save to API if logged in
            if (document.querySelector('.settings-toggle')) {
                savePreferencesToAPI({ accent_color: color });
            }
        });
    });
    
    // Feed toggle functionality
    const feedToggle = document.getElementById('feed-toggle-input');
    const feedTitle = document.getElementById('feed-title');
    
    if (feedToggle && feedTitle) {
        // Restore preference from localStorage
        const savedPreference = localStorage.getItem('followingFilterActive');
        if (savedPreference === 'true') {
            feedToggle.checked = true;
            filterActive = true;
            feedTitle.textContent = 'following feed';
            
            // Fetch following list and apply filter
            fetchFollowing().then(follows => {
                if (follows) {
                    followingDids = follows;
                    // Cache the following list with timestamp
                    localStorage.setItem('followingDids', JSON.stringify(follows));
                    localStorage.setItem('followingDidsTimestamp', Date.now().toString());
                    applyFollowingFilter(true);
                }
            });
        }
        
        feedToggle.addEventListener('change', async (e) => {
            filterActive = e.target.checked;
            localStorage.setItem('followingFilterActive', filterActive.toString());
            
            // Animate title change
            feedTitle.style.opacity = '0';
            
            setTimeout(() => {
                feedTitle.textContent = filterActive ? 'following feed' : 'global feed';
                feedTitle.style.opacity = '1';
            }, 150);
            
            if (filterActive) {
                // Check if we have cached following list (valid for 1 hour)
                const cachedFollows = localStorage.getItem('followingDids');
                const cacheTimestamp = localStorage.getItem('followingDidsTimestamp');
                const oneHour = 60 * 60 * 1000;
                
                if (cachedFollows && cacheTimestamp && 
                    (Date.now() - parseInt(cacheTimestamp)) < oneHour) {
                    // Use cached data
                    followingDids = JSON.parse(cachedFollows);
                } else {
                    // Fetch fresh data
                    const follows = await fetchFollowing();
                    if (follows) {
                        followingDids = follows;
                        // Cache the following list
                        localStorage.setItem('followingDids', JSON.stringify(follows));
                        localStorage.setItem('followingDidsTimestamp', Date.now().toString());
                    } else {
                        // Failed to fetch, disable filter
                        filterActive = false;
                        e.target.checked = false;
                        localStorage.setItem('followingFilterActive', 'false');
                        feedTitle.textContent = 'global feed';
                        alert('Failed to fetch following list');
                        return;
                    }
                }
            }
            
            applyFollowingFilter(filterActive);
        });
    }
    
    // Set up infinite scrolling
    window.addEventListener('scroll', checkScroll);
    
    // Check if we need to load more on initial page load
    // (in case the initial content doesn't fill the viewport)
    setTimeout(() => {
        if (document.documentElement.scrollHeight <= window.innerHeight) {
            loadMoreStatuses();
        }
    }, 100);
    
    // Admin hide button functionality
    document.querySelectorAll('.hide-button').forEach(button => {
        button.addEventListener('click', async (e) => {
            const uri = e.target.dataset.uri;
            if (!uri) return;
            
            if (!confirm('Hide this status from the global feed?')) {
                return;
            }
            
            try {
                const response = await fetch('/admin/hide-status', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({ 
                        uri: uri,
                        hidden: true 
                    })
                });
                
                const result = await response.json();
                
                if (response.ok) {
                    // Remove the status from the feed
                    e.target.closest('.status-item').style.display = 'none';
                } else {
                    alert(result.error || 'Failed to hide status');
                }
            } catch (error) {
                console.error('Error hiding status:', error);
                alert('Failed to hide status');
            }
        });
    });
};

    // Export for global use
    window.FeedManager = {
        init: initFeedPage
    };
})(); // End IIFE