// Emoji Resolver Module - Handles mapping emoji names to correct filenames
(function() {
    'use strict';
    
    // Cache for emoji name -> filename mapping
    let emojiMap = null;
    let loadPromise = null;
    
    // Load emoji mapping from API
    async function loadEmojiMap() {
        if (emojiMap) return emojiMap;
        if (loadPromise) return loadPromise;
        
        loadPromise = fetch('/api/custom-emojis')
            .then(response => response.json())
            .then(data => {
                if (!Array.isArray(data)) {
                    console.error('Invalid emoji data received');
                    return new Map();
                }
                emojiMap = new Map(data.map(emoji => [emoji.name, emoji.filename]));
                return emojiMap;
            })
            .catch(err => {
                console.error('Failed to load emoji map:', err);
                emojiMap = new Map();
                return emojiMap;
            });
        
        return loadPromise;
    }
    
    // Get the correct emoji filename for a given name
    function getEmojiFilename(emojiName) {
        if (!emojiMap) return null;
        return emojiMap.get(emojiName);
    }
    
    // Update a single emoji image element
    function updateEmojiImage(img) {
        const emojiName = img.getAttribute('data-emoji-name');
        if (!emojiName) return;
        
        const filename = getEmojiFilename(emojiName);
        if (filename) {
            // Found the correct filename, update src
            img.src = `/emojis/${filename}`;
            // Remove placeholder class if present
            img.classList.remove('emoji-placeholder');
            // Remove the error handler since we have the correct path
            img.onerror = null;
        } else {
            // Emoji not found in map, try common extensions as fallback
            // This handles newly added emojis that aren't in the cached map yet
            img.src = `/emojis/${emojiName}.png`;
            img.onerror = function() {
                this.onerror = null;
                this.src = `/emojis/${emojiName}.gif`;
            };
            img.classList.remove('emoji-placeholder');
        }
    }
    
    // Update all emoji images on the page
    function updateAllEmojiImages() {
        const images = document.querySelectorAll('img[data-emoji-name]');
        images.forEach(updateEmojiImage);
    }
    
    // Initialize on DOM ready
    async function initialize() {
        // Load the emoji map
        await loadEmojiMap();
        // Update all existing emoji images
        updateAllEmojiImages();
        
        // Set up a MutationObserver to handle dynamically added content
        const observer = new MutationObserver((mutations) => {
            mutations.forEach((mutation) => {
                mutation.addedNodes.forEach((node) => {
                    if (node.nodeType === Node.ELEMENT_NODE) {
                        // Check if the added node is an emoji image
                        if (node.tagName === 'IMG' && node.getAttribute('data-emoji-name')) {
                            updateEmojiImage(node);
                        }
                        // Also check descendants
                        const images = node.querySelectorAll?.('img[data-emoji-name]');
                        images?.forEach(updateEmojiImage);
                    }
                });
            });
        });
        
        // Start observing the document body for changes
        observer.observe(document.body, {
            childList: true,
            subtree: true
        });
    }
    
    // Export to global scope
    window.EmojiResolver = {
        loadEmojiMap,
        getEmojiFilename,
        updateEmojiImage,
        updateAllEmojiImages,
        initialize
    };
    
    // Auto-initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initialize);
    } else {
        // DOM is already ready
        initialize();
    }
})();