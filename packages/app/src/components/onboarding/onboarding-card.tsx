import type { ReactNode } from "react";
import { Text, View } from "react-native";

import { useTheme } from "@/styles/use-theme";

interface OnboardingCardProps {
  title: string;
  body?: string;
  children?: ReactNode;
  accent?: boolean;
}

/** Visually distinct card below the transcript — not message-shaped. */
export function OnboardingCard({ title, body, children, accent }: OnboardingCardProps) {
  const theme = useTheme();
  return (
    <View
      style={{
        marginTop: theme.spacing[4],
        padding: theme.spacing[4],
        borderRadius: theme.radius.xl,
        borderWidth: 1,
        borderColor: accent ? theme.colors.accent : theme.colors.borderAccent,
        borderStyle: "dashed",
        backgroundColor: theme.colors.surface1,
        gap: theme.spacing[3],
      }}
    >
      <View
        style={{
          alignSelf: "flex-start",
          paddingHorizontal: theme.spacing[2],
          paddingVertical: 2,
          borderRadius: theme.radius.sm,
          backgroundColor: accent ? theme.colors.surface3 : theme.colors.surface2,
        }}
      >
        <Text
          style={{
            color: accent ? theme.colors.accentBright : theme.colors.foregroundMuted,
            fontSize: theme.fontSize.xs,
            fontWeight: "700",
            letterSpacing: 0.4,
            textTransform: "uppercase",
          }}
        >
          Setup
        </Text>
      </View>
      <Text style={{ color: theme.colors.foreground, fontSize: theme.fontSize.base, fontWeight: "700" }}>
        {title}
      </Text>
      {body ? (
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm, lineHeight: 22 }}>
          {body}
        </Text>
      ) : null}
      {children}
    </View>
  );
}
