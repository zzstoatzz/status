document.addEventListener('DOMContentLoaded', () => {
  const tabs = document.querySelectorAll('#wh-lang-tabs [data-lang]');
  const blocks = document.querySelectorAll('.wh-snippet[data-lang]');
  if (!tabs.length || !blocks.length) return;
  const activate = (lang) => {
    tabs.forEach(t => t.classList.toggle('active', t.dataset.lang === lang));
    blocks.forEach(b => b.classList.toggle('active', b.dataset.lang === lang));
  };
  tabs.forEach(btn => btn.addEventListener('click', () => activate(btn.dataset.lang)));
  // default
  activate('node');
});

