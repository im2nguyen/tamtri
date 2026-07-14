import { Redirect, useLocalSearchParams } from "expo-router";

import { isSettingsSection } from "@/lib/settings-navigation";
import { SettingsScreen } from "@/screens/settings-screen";

export default function SettingsSectionRoute() {
  const params = useLocalSearchParams<{ section?: string; target?: string }>();
  const rawSection = typeof params.section === "string" ? params.section : "";
  const target = typeof params.target === "string" ? params.target : undefined;

  if (rawSection === "agents") {
    return <Redirect href="/settings/providers" />;
  }

  if (!isSettingsSection(rawSection)) {
    return <Redirect href="/settings/general" />;
  }

  return <SettingsScreen section={rawSection} target={target} />;
}
