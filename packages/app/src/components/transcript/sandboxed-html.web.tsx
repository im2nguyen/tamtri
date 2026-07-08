import { useEffect, useMemo } from "react";
import { View } from "react-native";

import { theme } from "@/styles/theme";

interface SandboxedHtmlProps {
  html: string;
  title?: string;
  height?: number;
  onNavigationBlocked?: (url: string) => void;
}

export function SandboxedHtml({ html, title, height = 420, onNavigationBlocked }: SandboxedHtmlProps) {
  const src = useMemo(() => {
    const blob = new Blob([html], { type: "text/html;charset=utf-8" });
    return URL.createObjectURL(blob);
  }, [html]);

  useEffect(() => {
    return () => URL.revokeObjectURL(src);
  }, [src]);

  return (
    <View
      style={{
        borderRadius: theme.radius.lg,
        overflow: "hidden",
        borderWidth: 1,
        borderColor: theme.colors.border,
        backgroundColor: theme.colors.surface0,
        height,
      }}
    >
      {/* Web-only sandboxed preview: no scripts, no navigation, no network. */}
      <iframe
        title={title ?? "Artifact preview"}
        src={src}
        sandbox=""
        style={{ width: "100%", height: "100%", border: "none", backgroundColor: "#fff" }}
        onLoad={(event) => {
          const frame = event.currentTarget;
          try {
            frame.contentWindow?.addEventListener("click", (clickEvent) => {
              const target = clickEvent.target as HTMLElement | null;
              const anchor = target?.closest("a");
              const href = anchor?.getAttribute("href");
              if (href && !href.startsWith("#")) {
                clickEvent.preventDefault();
                onNavigationBlocked?.(href);
              }
            });
          } catch {
            // Sandboxed iframe may block access; navigation is still restricted.
          }
        }}
      />
    </View>
  );
}
