// Beautiful timestamp formatting with hover tooltips
// Provides minute-resolution display by default with full timestamp on hover

const TimestampFormatter = {
    // Format a timestamp with appropriate granularity
    formatRelative(date, now = new Date()) {
        const diffMs = now - date;
        const diffSecs = Math.floor(diffMs / 1000);
        const diffMins = Math.floor(diffMs / 60000);
        const diffHours = Math.floor(diffMs / 3600000);
        const diffDays = Math.floor(diffMs / 86400000);
        
        // For very recent times, show "just now"
        if (diffSecs < 30) {
            return 'just now';
        }
        
        // Under 1 hour: show minutes
        if (diffMins < 60) {
            return `${diffMins}m ago`;
        }
        
        // Under 24 hours: show hours and minutes
        if (diffHours < 24) {
            const remainingMins = diffMins % 60;
            if (remainingMins === 0) {
                return `${diffHours}h ago`;
            }
            return `${diffHours}h ${remainingMins}m ago`;
        }
        
        // Under 7 days: show days and hours
        if (diffDays < 7) {
            const remainingHours = diffHours % 24;
            if (remainingHours === 0) {
                return `${diffDays}d ago`;
            }
            return `${diffDays}d ${remainingHours}h ago`;
        }
        
        // Over a week: show date with time
        const timeStr = date.toLocaleTimeString('en-US', {
            hour: 'numeric',
            minute: '2-digit',
            hour12: true
        }).toLowerCase();
        
        // If same year, don't show year
        if (date.getFullYear() === now.getFullYear()) {
            return date.toLocaleDateString('en-US', {
                month: 'short',
                day: 'numeric'
            }) + ', ' + timeStr;
        }
        
        // Different year: show full date
        return date.toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric',
            year: 'numeric'
        }) + ', ' + timeStr;
    },
    
    // Format future timestamps (for expiry times)
    formatFuture(date, now = new Date()) {
        const diffMs = date - now;
        const diffSecs = Math.floor(diffMs / 1000);
        const diffMins = Math.floor(diffMs / 60000);
        const diffHours = Math.floor(diffMs / 3600000);
        const diffDays = Math.floor(diffMs / 86400000);
        
        if (diffSecs < 60) {
            return 'expires soon';
        }
        
        if (diffMins < 60) {
            return `expires in ${diffMins}m`;
        }
        
        if (diffHours < 24) {
            const remainingMins = diffMins % 60;
            if (remainingMins === 0) {
                return `expires in ${diffHours}h`;
            }
            return `expires in ${diffHours}h ${remainingMins}m`;
        }
        
        if (diffDays < 7) {
            const remainingHours = diffHours % 24;
            if (remainingHours === 0) {
                return `expires in ${diffDays}d`;
            }
            return `expires in ${diffDays}d ${remainingHours}h`;
        }
        
        // Over a week: show date
        return 'expires ' + date.toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric',
            hour: 'numeric',
            minute: '2-digit',
            hour12: true
        }).toLowerCase();
    },
    
    // Format for history view (compact but informative)
    formatCompact(date, now = new Date()) {
        const diffMs = now - date;
        const diffMins = Math.floor(diffMs / 60000);
        const diffHours = Math.floor(diffMs / 3600000);
        const diffDays = Math.floor(diffMs / 86400000);
        
        // Today: show time only
        if (date.toDateString() === now.toDateString()) {
            return date.toLocaleTimeString('en-US', {
                hour: 'numeric',
                minute: '2-digit',
                hour12: true
            }).toLowerCase();
        }
        
        // Yesterday: show "yesterday" + time
        const yesterday = new Date(now);
        yesterday.setDate(yesterday.getDate() - 1);
        if (date.toDateString() === yesterday.toDateString()) {
            return 'yesterday, ' + date.toLocaleTimeString('en-US', {
                hour: 'numeric',
                minute: '2-digit',
                hour12: true
            }).toLowerCase();
        }
        
        // Within 7 days: show day of week + time
        if (diffDays < 7) {
            const dayName = date.toLocaleDateString('en-US', { weekday: 'short' }).toLowerCase();
            const time = date.toLocaleTimeString('en-US', {
                hour: 'numeric',
                minute: '2-digit',
                hour12: true
            }).toLowerCase();
            return `${dayName}, ${time}`;
        }
        
        // Same year: show month, day, time
        if (date.getFullYear() === now.getFullYear()) {
            return date.toLocaleDateString('en-US', {
                month: 'short',
                day: 'numeric',
                hour: 'numeric',
                minute: '2-digit',
                hour12: true
            }).toLowerCase();
        }
        
        // Different year: show full date
        return date.toLocaleDateString('en-US', {
            month: 'short',
            day: 'numeric',
            year: 'numeric',
            hour: 'numeric',
            minute: '2-digit',
            hour12: true
        }).toLowerCase();
    },
    
    // Get full timestamp for tooltip
    getFullTimestamp(date) {
        // Get day of week
        const dayName = date.toLocaleDateString('en-US', { weekday: 'long' });
        
        // Get month and day
        const monthDay = date.toLocaleDateString('en-US', {
            month: 'long',
            day: 'numeric',
            year: 'numeric'
        });
        
        // Get time with seconds
        const time = date.toLocaleTimeString('en-US', {
            hour: 'numeric',
            minute: '2-digit',
            second: '2-digit',
            hour12: true
        });
        
        // Get timezone
        const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
        const tzAbbr = date.toLocaleTimeString('en-US', {
            timeZoneName: 'short'
        }).split(' ').pop();
        
        return `${dayName}, ${monthDay} at ${time} ${tzAbbr}`;
    },
    
    // Initialize all timestamps on the page
    initialize() {
        const updateTimestamps = () => {
            const now = new Date();
            
            document.querySelectorAll('.local-time').forEach(el => {
                const timestamp = el.getAttribute('data-timestamp');
                if (!timestamp) return;
                
                const date = new Date(timestamp);
                const format = el.getAttribute('data-format');
                const prefix = el.getAttribute('data-prefix');
                
                let text = '';
                
                // Determine format type
                if (prefix === 'expires' || prefix === 'clears') {
                    text = this.formatFuture(date, now);
                } else if (format === 'compact' || format === 'short') {
                    text = this.formatCompact(date, now);
                } else if (prefix === 'since') {
                    const relativeText = this.formatRelative(date, now);
                    text = `since ${relativeText}`.replace('since just now', 'just started');
                } else {
                    text = this.formatRelative(date, now);
                }
                
                // Update text content
                el.textContent = text;
                
                // Add tooltip with full timestamp
                const fullTimestamp = this.getFullTimestamp(date);
                el.setAttribute('title', fullTimestamp);
                el.style.cursor = 'help';
                el.style.display = 'inline-block';
                el.style.lineHeight = '1.2';
                el.style.alignSelf = 'flex-start';
                el.style.width = 'auto';
            });
        };
        
        // Initial update
        updateTimestamps();
        
        // Update every 30 seconds for better granularity
        setInterval(updateTimestamps, 30000);
    }
};

// Auto-initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => TimestampFormatter.initialize());
} else {
    TimestampFormatter.initialize();
}