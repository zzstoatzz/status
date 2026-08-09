/**
 * The snippet you paste on your own site to show your current status.
 *
 * This existed before the hatk rewrite and was lost in the port — the toggle
 * button came across, the panel did not — so it is rebuilt here, with the one
 * bug the original had fixed: it fetched from a hardcoded
 * `pds.zzstoatzz.io`, so it only ever worked for that one account. It reads
 * from this appview instead, which knows every indexed repo regardless of where
 * it is hosted.
 *
 * Deliberately a self-contained script with no build step, no dependency, and
 * nothing for us to keep serving beyond the API it already exposes. It degrades
 * to rendering nothing rather than throwing on someone else's page.
 */

const BUFO = "https://find-bufo.com/e";

/** Quote a value for single-quoted javascript inside an HTML snippet. */
function js(value: string): string {
  return value.replace(/\\/g, "\\\\").replace(/'/g, "\\'").replace(/</g, "\\x3c");
}

export function buildEmbedCode(opts: { did: string; handle?: string; origin: string }): string {
  const did = js(opts.did);
  const origin = js(opts.origin.replace(/\/$/, ""));
  const profile = js(`${opts.origin.replace(/\/$/, "")}/@${opts.handle ?? opts.did}`);

  // `</` is split so the snippet can be pasted inside another script tag, and
  // because writing it literally would close this one when it is inlined.
  return `<div id="status-embed"></div>
<script>
(function () {
  var el = document.getElementById('status-embed');
  if (!el) return;
  fetch('${origin}/xrpc/dev.hatk.getFeed?feed=actor&actor=${did}&limit=1')
    .then(function (r) { return r.json() })
    .then(function (d) {
      var s = d && d.items && d.items[0];
      if (!s) return;
      var emoji = s.emoji || '';
      var glyph = emoji.indexOf('custom:') === 0
        ? '<img src="${BUFO}/' + encodeURIComponent(emoji.slice(7)) + '.png" alt="" ' +
          'style="width:1.25em;height:1.25em;vertical-align:-0.25em">'
        : emoji;
      var a = document.createElement('a');
      a.href = '${profile}';
      a.target = '_blank';
      a.rel = 'noopener';
      a.style.cssText = 'text-decoration:none;color:inherit';
      a.innerHTML = glyph + ' ';
      a.appendChild(document.createTextNode(s.text || ''));
      el.innerHTML = '';
      el.appendChild(a);
    })
    .catch(function () {});
})();
</${""}script>`;
}
