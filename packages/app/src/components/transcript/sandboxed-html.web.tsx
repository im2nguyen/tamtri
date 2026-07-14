import { useEffect, useMemo } from "react";
import { Pressable, Text, View } from "react-native";

import { collectBlockedHrefs, prepareArtifactHtml } from "@/components/transcript/sandboxed-html-prep.web";
import { useTheme } from "@/styles/use-theme";

interface SandboxedHtmlProps {
  html: string;
  title?: string;
  height?: number;
  fill?: boolean;
  onNavigationBlocked?: (url: string) => void;
}

export function SandboxedHtml({ html, title, height = 420, fill = false, onNavigationBlocked }: SandboxedHtmlProps) {
  const theme = useTheme();
  const blockedHrefs = useMemo(() => collectBlockedHrefs(html), [html]);
  const src = useMemo(() => {
    const blob = new Blob([prepareArtifactHtml(html)], { type: "text/html;charset=utf-8" });
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
        flexDirection: "column",
        ...(fill ? { flex: 1, minHeight: 200 } : { height }),
      }}
    >
      {/* Web-only sandboxed preview: no scripts, no navigation, no network. */}
      <iframe
        title={title ?? "Artifact preview"}
        src={src}
        sandbox=""
        style={{ width: "100%", height: "100%", border: "none", backgroundColor: "#fff", flex: 1 }}
      />
      {blockedHrefs.length > 0 ? (
        <View
          style={{
            borderTopWidth: 1,
            borderTopColor: theme.colors.border,
            paddingHorizontal: theme.spacing[3],
            paddingVertical: theme.spacing[2],
            gap: theme.spacing[1],
            backgroundColor: theme.colors.surface1,
          }}
        >
          <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, fontWeight: "600" }}>
            Blocked links
          </Text>
          {blockedHrefs.map((url) => (
            <Pressable
              key={url}
              accessibilityRole="link"
              onPress={() => onNavigationBlocked?.(url)}
            >
              <Text
                style={{
                  color: theme.colors.accentBright,
                  fontSize: theme.fontSize.xs,
                  fontFamily: theme.fontFamily.mono,
                }}
                numberOfLines={2}
              >
                {url}
              </Text>
            </Pressable>
          ))}
        </View>
      ) : null}
    </View>
  );
}
