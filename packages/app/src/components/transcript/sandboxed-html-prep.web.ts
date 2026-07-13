const ARTIFACT_CSP =
  "default-src 'none'; img-src data: blob:; media-src data: blob:; font-src data:; style-src 'unsafe-inline'; connect-src 'none'; frame-src 'none'; form-action 'none'; base-uri 'none';";

const ACTIVE_CONTENT_SELECTORS =
  "script, iframe, embed, object, frame, frameset, meta[http-equiv='Content-Security-Policy' i], meta[http-equiv='refresh' i]";

export function prepareArtifactHtml(html: string): string {
  const document = new DOMParser().parseFromString(html, "text/html");
  document.querySelectorAll(ACTIVE_CONTENT_SELECTORS).forEach((node) => node.remove());
  document.querySelectorAll("base").forEach((node) => node.remove());
  for (const node of Array.from(document.querySelectorAll<HTMLElement>("*"))) {
    for (const attribute of ["src", "href", "action", "formaction", "poster", "srcset"]) {
      const value = node.getAttribute(attribute)?.trim();
      if (!value) continue;
      const allowed =
        (attribute === "href" && value.startsWith("#")) ||
        value.startsWith("data:") ||
        value.startsWith("blob:");
      if (!allowed && attribute === "href" && node.tagName === "A") {
        node.setAttribute("data-tamtri-blocked-href", value);
        node.setAttribute("href", "#");
      } else if (!allowed) {
        node.removeAttribute(attribute);
      }
    }
  }
  const csp = document.createElement("meta");
  csp.httpEquiv = "Content-Security-Policy";
  csp.content = ARTIFACT_CSP;
  document.head.prepend(csp);
  return `<!doctype html>\n${document.documentElement.outerHTML}`;
}

export function collectBlockedHrefs(html: string): string[] {
  const document = new DOMParser().parseFromString(prepareArtifactHtml(html), "text/html");
  const hrefs = new Set<string>();
  for (const node of Array.from(document.querySelectorAll<HTMLElement>("[data-tamtri-blocked-href]"))) {
    const href = node.getAttribute("data-tamtri-blocked-href")?.trim();
    if (href) hrefs.add(href);
  }
  return [...hrefs];
}
