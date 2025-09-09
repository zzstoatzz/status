// Lightweight markdown link renderer for status text
// Converts [text](url) into <a href> with basic sanitization
(function () {
  const MD_LINK_RE = /\[([^\]]+)\]\(([^)]+)\)/g;

  function escapeHtml(str) {
    return String(str)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  function normalizeUrl(url) {
    let u = url.trim();
    // If no scheme and looks like a domain, prefix with https://
    if (!/^([a-z]+:)?\/\//i.test(u)) {
      u = 'https://' + u;
    }
    try {
      const parsed = new URL(u);
      if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
        return parsed.toString();
      }
      return null; // disallow other protocols
    } catch (_) {
      return null;
    }
  }

  function linkifyMarkdown(text) {
    return text.replace(MD_LINK_RE, (_m, label, url) => {
      const safeUrl = normalizeUrl(url);
      const safeLabel = escapeHtml(label);
      if (!safeUrl) return `[${safeLabel}](${escapeHtml(url)})`;
      return `<a href="${safeUrl}" target="_blank" rel="noopener noreferrer nofollow">${safeLabel}</a>`;
    });
  }

  function renderMarkdownLinksIn(root) {
    const scope = root || document;
    const nodes = scope.querySelectorAll('.status-text:not([data-md-rendered]), .history-text:not([data-md-rendered])');
    nodes.forEach((el) => {
      const original = el.textContent || '';
      const rendered = linkifyMarkdown(original);
      if (rendered !== original) {
        el.innerHTML = rendered;
      }
      el.setAttribute('data-md-rendered', 'true');
    });
  }

  // Expose globally
  window.renderMarkdownLinksIn = renderMarkdownLinksIn;
})();

