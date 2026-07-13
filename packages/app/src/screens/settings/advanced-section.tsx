import { ActivityIndicator, Text, View } from "react-native";

import {
  SettingsCard,
  SettingsListRow,
  SettingsRow,
  SettingsSection,
} from "@/components/settings/settings-section";
import { Button } from "@/components/ui/button";
import { useVaultIssues } from "@/hooks/use-vault-issues";
import { useTheme } from "@/styles/use-theme";

export function AdvancedSection() {
  const theme = useTheme();
  const { issues, loading, error, refresh } = useVaultIssues();

  return (
    <SettingsSection title="Vault diagnostics">
      <SettingsCard>
        <SettingsRow
          title="Vault health"
          description="Ask the daemon to scan for duplicate IDs and malformed conversations."
          control={
            <Button
              label={loading ? "Checking…" : "Refresh"}
              compact
              variant="secondary"
              disabled={loading}
              onPress={() => void refresh()}
            />
          }
        >
          {loading ? (
            <ActivityIndicator
              color={theme.colors.accentBright}
              style={{ alignSelf: "flex-start", marginTop: theme.spacing[3] }}
            />
          ) : error ? (
            <Text
              style={{
                color: theme.colors.destructive,
                fontSize: theme.fontSize.xs,
                marginTop: theme.spacing[3],
              }}
            >
              {error}
            </Text>
          ) : issues.length === 0 ? (
            <Text
              style={{
                color: theme.colors.foregroundMuted,
                fontSize: theme.fontSize.xs,
                marginTop: theme.spacing[3],
              }}
            >
              No vault issues reported.
            </Text>
          ) : null}
        </SettingsRow>
        {issues.map((issue, index) => (
          <SettingsListRow
            key={`${issue.kind}-${issue.detail}`}
            divided
            title={issue.kind}
            description={
              <View>
                <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.xs }}>
                  {issue.detail}
                </Text>
              </View>
            }
          />
        ))}
      </SettingsCard>
    </SettingsSection>
  );
}
