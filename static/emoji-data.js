// Fetch emoji data from CDN
// Using emoji-datasource which provides comprehensive emoji data with search keywords
async function loadEmojiData() {
    try {
        console.log('Loading emoji data from CDN...');
        // Using jsdelivr CDN for emoji-datasource-apple (or could use google/twitter/facebook)
        const response = await fetch('https://cdn.jsdelivr.net/npm/emoji-datasource@15.1.0/emoji.json');
        if (!response.ok) {
            throw new Error(`Failed to fetch emoji data: ${response.status}`);
        }
        const emojiData = await response.json();
        console.log(`Loaded ${emojiData.length} emojis from CDN`);
        
        // Transform into a simpler format for our needs
        const emojis = {}; // char -> keywords[]
        const slugs = {};  // char -> slug (first short_name fallback from name)
        const reserved = new Set(); // all slugs
        const categories = {
            frequent: ['😊', '👍', '❤️', '😂', '🎉', '🔥', '✨', '💯', '🚀', '💪', '🙏', '👏'],
            people: [],
            nature: [],
            food: [],
            activity: [],
            travel: [],
            objects: [],
            symbols: [],
            flags: []
        };
        
        emojiData.forEach(emoji => {
            // Get the actual emoji character
            const char = emoji.unified.split('-').map(u => String.fromCodePoint(parseInt(u, 16))).join('');
            
            // Build search keywords from short_names and text
            const keywords = [
                ...(emoji.short_names || []),
                ...(emoji.name ? emoji.name.toLowerCase().split(' ') : [])
            ].flat();
            
            // Add the name itself as keywords
            if (emoji.name) {
                keywords.push(...emoji.name.toLowerCase().split(/[\s_-]+/));
            }
            
            // Add any additional search terms from the texts field
            if (emoji.texts) {
                keywords.push(...emoji.texts);
            }
            
            emojis[char] = keywords;

            // Pick a slug: prefer the first short_name
            let slug = null;
            if (emoji.short_names && emoji.short_names.length > 0) {
                slug = emoji.short_names[0].toLowerCase();
            } else if (emoji.name) {
                slug = emoji.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
            }
            if (slug) {
                slugs[char] = slug;
                reserved.add(slug);
            }
            
            // Add to category
            const categoryMap = {
                'Smileys & Emotion': 'people',
                'People & Body': 'people', 
                'Animals & Nature': 'nature',
                'Food & Drink': 'food',
                'Activities': 'activity',
                'Travel & Places': 'travel',
                'Objects': 'objects',
                'Symbols': 'symbols',
                'Flags': 'flags'
            };
            
            const category = categoryMap[emoji.category];
            if (category && categories[category]) {
                categories[category].push(char);
            }
        });
        
        console.log(`Built emoji database with ${Object.keys(emojis).length} emojis`);
        return { emojis, categories, slugs, reserved: Array.from(reserved) };
    } catch (error) {
        console.error('Failed to load emoji data:', error);
        // Fallback to a minimal set if the CDN fails
        return {
            emojis: {
                '😊': ['smile', 'happy'],
                '👍': ['thumbs up', 'good'],
                '❤️': ['heart', 'love'],
                '😂': ['laugh', 'lol'],
                '🎉': ['party', 'celebrate']
            },
            categories: {
                frequent: ['😊', '👍', '❤️', '😂', '🎉'],
                people: ['😊', '😂'],
                nature: [],
                food: [],
                activity: [],
                travel: [],
                objects: [],
                symbols: ['❤️'],
                flags: []
            },
            slugs: {
                '😊': 'smile',
                '👍': 'thumbsup',
                '❤️': 'heart',
                '😂': 'joy',
                '🎉': 'tada'
            },
            reserved: ['smile','thumbsup','heart','joy','tada']
        };
    }
}

// Export for use in the main page
window.emojiDataLoader = { loadEmojiData };
