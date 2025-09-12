// Admin Panel Functionality

// Initialize admin upload functionality
const initAdminPanel = () => {
    const toggle = document.getElementById('admin-toggle');
    const content = document.getElementById('admin-content');
    const form = document.getElementById('emoji-upload-form');
    const file = document.getElementById('emoji-file');
    const name = document.getElementById('emoji-name');
    const msg = document.getElementById('admin-msg');
    
    if (!toggle || !content || !form) return;

    toggle.addEventListener('click', () => {
        content.style.display = content.style.display === 'none' ? 'block' : 'none';
    });

    form.addEventListener('submit', async (e) => {
        e.preventDefault();
        msg.textContent = '';
        
        if (!file.files || file.files.length === 0) {
            msg.textContent = 'choose a PNG or GIF';
            return;
        }
        
        // Require a name; prefill from filename if empty
        if (!name.value.trim().length) {
            const base = (file.files[0].name || '').replace(/\.[^.]+$/, '');
            const sanitized = base.toLowerCase().replace(/[^a-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '');
            name.value = sanitized || '';
        }
        
        if (!name.value.trim().length) {
            msg.textContent = 'please choose a name';
            return;
        }
        
        // Client-side reserved check (best-effort)
        if (window.__reservedEmojiNames && window.__reservedEmojiNames.has(name.value.trim().toLowerCase())) {
            msg.textContent = 'that name is reserved by a standard emoji';
            return;
        }
        
        const f = file.files[0];
        if (!['image/png','image/gif'].includes(f.type)) {
            msg.textContent = 'only PNG or GIF';
            return;
        }
        
        const fd = new FormData();
        fd.append('file', f);
        if (name.value.trim().length) fd.append('name', name.value.trim());
        
        try {
            const res = await fetch('/admin/upload-emoji', { method: 'POST', body: fd });
            const json = await res.json();
            
            if (!res.ok || !json.success) {
                if (json && json.code === 'name_exists') {
                    msg.textContent = 'that name already exists — please pick another';
                } else {
                    msg.textContent = (json && json.error) || 'upload failed';
                }
                return;
            }
            
            // Notify listeners (e.g., emoji picker) and close panel
            document.dispatchEvent(new CustomEvent('custom-emoji-uploaded', { detail: json }));
            content.style.display = 'none';
            form.reset();
            msg.textContent = '';
        } catch (err) {
            msg.textContent = 'network error';
        }
    });
};

// Export for global use
window.AdminManager = {
    init: initAdminPanel
};