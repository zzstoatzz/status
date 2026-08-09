import { describe, expect, it } from "vitest";
import { buildEmbedCode } from "./embed.ts";

const DID = "did:plc:xbtmt2zjwlrfegqvch7fboei";
const ORIGIN = "https://status.zzstoatzz.io";

describe("buildEmbedCode", () => {
  const code = buildEmbedCode({ did: DID, handle: "zzstoatzz.io", origin: ORIGIN });

  /**
   * The bug in the version this replaces: it fetched from a hardcoded
   * pds.zzstoatzz.io, so the snippet only worked for one account.
   */
  it("reads from the appview, not from one hardcoded pds", () => {
    expect(code).toContain(`${ORIGIN}/xrpc/dev.hatk.getFeed?feed=actor&actor=${DID}&limit=1`);
    expect(code).not.toContain("pds.zzstoatzz.io");
  });

  it("links back to the author's page", () => {
    expect(code).toContain(`${ORIGIN}/@zzstoatzz.io`);
  });

  it("falls back to the did when there is no handle", () => {
    const c = buildEmbedCode({ did: DID, origin: ORIGIN });
    expect(c).toContain(`${ORIGIN}/@${DID}`);
  });

  it("tolerates a trailing slash on the origin", () => {
    const c = buildEmbedCode({ did: DID, handle: "a.test", origin: `${ORIGIN}/` });
    expect(c).not.toContain("//xrpc");
    expect(c).toContain(`${ORIGIN}/@a.test`);
  });

  it("is a complete, pasteable snippet", () => {
    expect(code.startsWith('<div id="status-embed"></div>')).toBe(true);
    expect(code.trimEnd().endsWith("</script>")).toBe(true);
    // exactly one script block: an extra closer would break the host page
    expect(code.match(/<\/script>/g)?.length).toBe(1);
  });

  it("escapes quotes and angle brackets so a handle cannot break out", () => {
    const c = buildEmbedCode({
      did: DID,
      handle: "evil'</script><script>alert(1)//",
      origin: ORIGIN,
    });
    expect(c).not.toContain("</script><script>");
    expect(c).not.toContain("@evil'</");
  });

  it("renders status text as a text node rather than markup", () => {
    // the text is user-controlled and goes onto someone else's page
    expect(code).toContain("createTextNode(s.text");
    expect(code).not.toContain("innerHTML = glyph + ' ' + s.text");
  });

  it("resolves a custom bufo through the resolver the app already uses", () => {
    expect(code).toContain("https://find-bufo.com/e/");
    expect(code).toContain("encodeURIComponent(emoji.slice(7))");
  });
});
