(function () {
    const FEEDBACK_DURATION = 2200;

    function cacheDefaults(button) {
        if (!button.dataset.defaultLabel) {
            const labelEl = button.querySelector('.share-label');
            if (labelEl) {
                const text = labelEl.textContent ? labelEl.textContent.trim() : '';
                labelEl.dataset.defaultLabel = text;
                button.dataset.defaultLabel = text;
            } else {
                button.dataset.defaultLabel = button.textContent.trim();
            }
        }
    }

    function setLabel(button, message) {
        const labelEl = button.querySelector('.share-label');
        if (labelEl) {
            labelEl.textContent = message;
        } else {
            button.textContent = message;
        }
    }

    function restoreLabel(button) {
        const labelEl = button.querySelector('.share-label');
        if (labelEl && labelEl.dataset.defaultLabel) {
            labelEl.textContent = labelEl.dataset.defaultLabel;
        } else if (button.dataset.defaultLabel) {
            button.textContent = button.dataset.defaultLabel;
        }
        button.classList.remove('share-trigger--feedback');
    }

    function scheduleRestore(button) {
        if (button.dataset.shareFeedbackTimer) {
            clearTimeout(Number(button.dataset.shareFeedbackTimer));
        }
        const timer = window.setTimeout(() => {
            restoreLabel(button);
            delete button.dataset.shareFeedbackTimer;
        }, FEEDBACK_DURATION);
        button.dataset.shareFeedbackTimer = String(timer);
    }

    async function copyToClipboard(text) {
        if (navigator.clipboard && navigator.clipboard.writeText) {
            await navigator.clipboard.writeText(text);
            return true;
        }
        const textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.setAttribute('readonly', '');
        textarea.style.position = 'absolute';
        textarea.style.left = '-9999px';
        document.body.appendChild(textarea);
        textarea.select();
        try {
            const success = document.execCommand('copy');
            document.body.removeChild(textarea);
            return success;
        } catch (err) {
            document.body.removeChild(textarea);
            return false;
        }
    }

    async function handleShare(button) {
        cacheDefaults(button);

        const sharePath = button.dataset.sharePath;
        if (!sharePath) {
            return;
        }
        const shareUrl = new URL(sharePath, window.location.origin).toString();
        const shareTitle = button.dataset.shareTitle || '';
        const shareText = button.dataset.shareText || '';

        if (navigator.share) {
            try {
                await navigator.share({
                    title: shareTitle || undefined,
                    text: shareText || undefined,
                    url: shareUrl,
                });
                button.classList.add('share-trigger--feedback');
                setLabel(button, 'shared!');
                scheduleRestore(button);
                return;
            } catch (err) {
                if (err && err.name === 'AbortError') {
                    return;
                }
                // fall through to clipboard copy
            }
        }

        try {
            const copied = await copyToClipboard(shareUrl);
            if (copied) {
                button.classList.add('share-trigger--feedback');
                setLabel(button, 'copied!');
                scheduleRestore(button);
                return;
            }
        } catch (err) {
            // ignore and fall back to opening link
        }

        window.open(shareUrl, '_blank', 'noopener');
        button.classList.add('share-trigger--feedback');
        setLabel(button, 'opened');
        scheduleRestore(button);
    }

    document.addEventListener('DOMContentLoaded', function () {
        document.querySelectorAll('.share-trigger').forEach(cacheDefaults);
    });

    document.addEventListener('click', function (event) {
        const button = event.target.closest('.share-trigger');
        if (!button) {
            return;
        }
        event.preventDefault();
        handleShare(button);
    });
})();
