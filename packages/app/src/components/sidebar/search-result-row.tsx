import { Pressable, Text, View } from "react-native";

import type { SearchHit } from "@/lib/daemon-types";
import { useTheme } from "@/styles/use-theme";

interface SearchResultRowProps {
  hit: SearchHit;
  selected?: boolean;
  onPress: () => void;
}

export function SearchResultRow({ hit, selected, onPress }: SearchResultRowProps) {
  const theme = useTheme();
  return (
    <Pressable
      onPress={onPress}
      style={({ pressed }) => ({
        paddingHorizontal: theme.spacing[3],
        paddingVertical: theme.spacing[3],
        borderRadius: theme.radius.lg,
        backgroundColor: selected
          ? theme.colors.surface2
          : pressed
            ? theme.colors.surfaceSidebarHover
            : "transparent",
        gap: theme.spacing[1],
      })}
    >
      <Text numberOfLines={1} style={{ color: theme.colors.foreground, fontSize: theme.fontSize.sm, fontWeight: "600" }}>
        {hit.title}
      </Text>
      <Text numberOfLines={2} style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs, lineHeight: 18 }}>
        {hit.snippet}
      </Text>
      <View style={{ flexDirection: "row", gap: theme.spacing[2], marginTop: 2 }}>
        <Text style={{ color: theme.colors.accentBright, fontSize: theme.fontSize.xs, fontWeight: "700", textTransform: "uppercase" }}>
          {hit.match_field}
        </Text>
      </View>
    </Pressable>
  );
}
