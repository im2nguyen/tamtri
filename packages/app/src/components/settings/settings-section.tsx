import { useMemo, type ReactNode } from "react";
import { Text, View, type StyleProp, type ViewStyle } from "react-native";

import { useTheme } from "@/styles/use-theme";
import { settingRowAnchorId } from "@/lib/settings-navigation";

export function useSettingsStyles() {
  const theme = useTheme();
  return useMemo(
    () => ({
      section: {
        marginBottom: theme.spacing[6],
      } as ViewStyle,
      sectionHeaderTitle: {
        color: theme.colors.foregroundMuted,
        fontSize: theme.fontSize.xs,
        fontWeight: "500" as const,
        marginBottom: theme.spacing[3],
        marginLeft: theme.spacing[1],
      },
      card: {
        backgroundColor: "transparent",
        borderRadius: theme.radius.lg,
        borderWidth: theme.hairlineWidth,
        borderColor: theme.colors.border,
        overflow: "hidden" as const,
      } as ViewStyle,
      row: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        justifyContent: "space-between" as const,
        paddingVertical: theme.density.settingsRowPaddingY,
        paddingHorizontal: theme.spacing[3],
      } as ViewStyle,
      rowWithBorder: {
        flexDirection: "row" as const,
        alignItems: "center" as const,
        justifyContent: "space-between" as const,
        paddingVertical: theme.density.settingsRowPaddingY,
        paddingHorizontal: theme.spacing[3],
        borderTopWidth: theme.hairlineWidth,
        borderTopColor: theme.colors.border,
      } as ViewStyle,
      rowContent: {
        flex: 1,
        marginRight: theme.spacing[3],
      } as ViewStyle,
      rowTitle: {
        color: theme.colors.foreground,
        fontSize: theme.fontSize.base,
      },
      rowHint: {
        color: theme.colors.foregroundMuted,
        fontSize: theme.fontSize.xs,
        marginTop: theme.spacing[1],
      },
    }),
    [theme],
  );
}

interface SettingsSectionProps {
  title: string;
  children: ReactNode;
  style?: StyleProp<ViewStyle>;
}

export function SettingsSection({ title, children, style }: SettingsSectionProps) {
  const settingsStyles = useSettingsStyles();
  return (
    <View style={[settingsStyles.section, style]}>
      <Text style={settingsStyles.sectionHeaderTitle}>{title}</Text>
      {children}
    </View>
  );
}

export function SettingsCard({
  children,
  style,
}: {
  children: ReactNode;
  style?: StyleProp<ViewStyle>;
}) {
  const settingsStyles = useSettingsStyles();
  return <View style={[settingsStyles.card, style]}>{children}</View>;
}

export interface SettingsRowProps {
  title: ReactNode;
  description?: ReactNode;
  control?: ReactNode;
  children?: ReactNode;
  divided?: boolean;
  align?: "center" | "start";
}

export function SettingsRow({
  title,
  description,
  control,
  children,
  divided,
  align = "center",
}: SettingsRowProps) {
  const styles = useSettingsStyles();
  const anchorId = typeof title === "string" ? settingRowAnchorId(title) : undefined;
  return (
    <View
      nativeID={anchorId}
      style={[
        divided ? styles.rowWithBorder : styles.row,
        { flexDirection: "column", alignItems: "stretch" },
      ]}
    >
      <View
        style={{
          flex: 1,
          flexDirection: "row",
          alignItems: align === "start" ? "flex-start" : "center",
          justifyContent: "space-between",
        }}
      >
        <View style={styles.rowContent}>
          {typeof title === "string" ? <Text style={styles.rowTitle}>{title}</Text> : title}
          {description != null ? (
            typeof description === "string" ? (
              <Text style={styles.rowHint}>{description}</Text>
            ) : (
              description
            )
          ) : null}
        </View>
        {control}
      </View>
      {children}
    </View>
  );
}

/** Dynamic collection row with the same geometry as a setting row. */
export const SettingsListRow = SettingsRow;
