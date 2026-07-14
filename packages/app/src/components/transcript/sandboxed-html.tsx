import { Platform, Text, View } from "react-native";

import { SandboxedHtml } from "@/components/transcript/sandboxed-html.web";
import { useTheme } from "@/styles/use-theme";

interface SandboxedHtmlProps {
  html: string;
  title?: string;
  height?: number;
  fill?: boolean;
  onNavigationBlocked?: (url: string) => void;
}

export function SandboxedHtmlNative(props: SandboxedHtmlProps) {
  const theme = useTheme();
  if (Platform.OS === "web") {
    return <SandboxedHtml {...props} />;
  }
  return (
    <View style={{ padding: theme.spacing[3], backgroundColor: theme.colors.surface1, borderRadius: theme.radius.lg }}>
      <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>
        Inline HTML preview is available in the desktop and web shell.
      </Text>
    </View>
  );
}

export { SandboxedHtmlNative as SandboxedHtml };
