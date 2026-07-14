import { ScrollViewStyleReset } from "expo-router/html";
import { type PropsWithChildren } from "react";

import {
  DEFAULT_MONO_FONT_STACK,
  DEFAULT_UI_FONT_STACK,
  GEIST_GOOGLE_FONTS_URL,
} from "@/styles/tokens";

const INITIAL_APPEARANCE_CSS = `
:root {
  --tamtri-font-ui: ${DEFAULT_UI_FONT_STACK};
  --tamtri-font-mono: ${DEFAULT_MONO_FONT_STACK};
  --tamtri-font-size-base: 16px;
  --tamtri-font-size-code: 13px;
}
#root {
  font-family: var(--tamtri-font-ui);
  font-size: var(--tamtri-font-size-base);
}
#root *:not([data-tamtri-mono="true"]) {
  font-family: inherit !important;
}
#root [data-tamtri-mono="true"] {
  font-family: var(--tamtri-font-mono) !important;
}
#root *[style*="ui-monospace"],
#root *[style*="SFMono"],
#root *[style*="Consolas"],
#root *[style*="Menlo"],
#root *[style*="monospace"] {
  font-family: var(--tamtri-font-mono) !important;
}
`.trim();

/** Web document shell — loads Geist before the app bundle paints. */
export default function Root({ children }: PropsWithChildren) {
  return (
    <html lang="en">
      <head>
        <meta charSet="utf-8" />
        <meta httpEquiv="X-UA-Compatible" content="IE=edge" />
        <ScrollViewStyleReset />
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="" />
        <link href={GEIST_GOOGLE_FONTS_URL} rel="stylesheet" />
        <style id="tamtri-appearance" dangerouslySetInnerHTML={{ __html: INITIAL_APPEARANCE_CSS }} />
      </head>
      <body>{children}</body>
    </html>
  );
}
