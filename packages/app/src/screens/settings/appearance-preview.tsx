import { useMemo } from "react";
import { Platform, Text, View } from "react-native";

import { useAppearanceStore } from "@/stores/appearance-store";
import { DEFAULT_MONO_FONT_STACK } from "@/styles/tokens";
import { syntaxThemeById } from "@/styles/syntax-themes";
import { useTheme } from "@/styles/use-theme";
import {
  CHANGED_LINE_INDICES,
  PREVIEW_AFTER,
  PREVIEW_BEFORE,
} from "@/screens/settings/preview-snippet";
import { tokenizeLines } from "@/screens/settings/tokenize-preview";

const ZERO_WIDTH = "\u200b";

type RowType = "context" | "add" | "remove";

interface PreviewOverrides {
  monoFontFamily?: string;
  codeFontSize?: number;
}

interface AppearancePreviewProps {
  overrides?: PreviewOverrides;
}

interface UnifiedRow {
  key: string;
  type: RowType;
  marker: string;
  tokens: ReturnType<typeof tokenizeLines>[number];
}

function buildUnifiedRows(): UnifiedRow[] {
  const beforeLines = tokenizeLines(PREVIEW_BEFORE);
  const afterLines = tokenizeLines(PREVIEW_AFTER);
  const rows: UnifiedRow[] = [];

  for (let index = 0; index < PREVIEW_BEFORE.length; index += 1) {
    if (CHANGED_LINE_INDICES.has(index)) {
      rows.push({
        key: `r-${index}`,
        type: "remove",
        marker: "- ",
        tokens: beforeLines[index] ?? [],
      });
      rows.push({
        key: `a-${index}`,
        type: "add",
        marker: "+ ",
        tokens: afterLines[index] ?? [],
      });
    } else {
      rows.push({
        key: `c-${index}`,
        type: "context",
        marker: "  ",
        tokens: beforeLines[index] ?? [],
      });
    }
  }

  return rows;
}

export function AppearancePreview({ overrides }: AppearancePreviewProps) {
  const theme = useTheme();
  const syntaxTheme = useAppearanceStore((s) => s.syntaxTheme);
  const syntaxColors = syntaxThemeById(syntaxTheme).colors;
  const rows = useMemo(() => buildUnifiedRows(), []);

  const monoFamily =
    overrides?.monoFontFamily?.trim() || theme.fontFamily.mono || DEFAULT_MONO_FONT_STACK;
  const codeSize = overrides?.codeFontSize ?? theme.fontSize.code;
  const lineHeight = Math.round(codeSize * 1.5);

  return (
    <View
      accessibilityRole="image"
      accessibilityLabel="Syntax highlight preview"
      style={{
        backgroundColor: theme.colors.surface1,
        borderRadius: theme.radius.lg,
        borderWidth: 1,
        borderColor: theme.colors.border,
        overflow: "hidden",
        paddingVertical: theme.spacing[2],
      }}
    >
      {rows.map((row) => {
        const rowBg =
          row.type === "add"
            ? theme.colors.diffAddTint
            : row.type === "remove"
              ? theme.colors.diffRemoveTint
              : "transparent";
        const markerColor =
          row.type === "add"
            ? theme.colors.diffAddition
            : row.type === "remove"
              ? theme.colors.diffDeletion
              : theme.colors.foregroundMuted;

        return (
          <View
            key={row.key}
            style={{
              paddingHorizontal: theme.spacing[3],
              backgroundColor: rowBg,
            }}
          >
            <Text
              style={{
                fontFamily: monoFamily,
                fontSize: codeSize,
                lineHeight,
                color: syntaxColors.plain,
                ...(Platform.OS === "web" ? { whiteSpace: "pre" as const } : null),
              }}
            >
              <Text style={{ color: markerColor }}>{row.marker}</Text>
              {row.tokens.length > 0
                ? row.tokens.map((token, tokenIndex) => (
                    <Text key={`${row.key}-${tokenIndex}`} style={{ color: syntaxColors[token.style] }}>
                      {token.text}
                    </Text>
                  ))
                : ZERO_WIDTH}
            </Text>
          </View>
        );
      })}
    </View>
  );
}
