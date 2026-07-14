import { View } from "react-native";

import {
  SettingsCard,
  SettingsRow,
  SettingsSection,
} from "@/components/settings/settings-section";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { useAppearanceStore } from "@/stores/appearance-store";
import { UI_DENSITIES, type UiDensity } from "@/styles/density";

const DENSITY_OPTIONS = UI_DENSITIES.map((value) => ({
  value,
  label: value[0]!.toUpperCase() + value.slice(1),
}));

export function GeneralSection() {
  const density = useAppearanceStore((state) => state.density);
  const setDensity = useAppearanceStore((state) => state.setDensity);

  return (
    <View>
      <SettingsSection title="Interface">
        <SettingsCard>
          <SettingsRow
            title="Interface density"
            description="Adjust rows, gutters, composer spacing, and settings without changing text size."
            control={
              <SegmentedControl<UiDensity>
                value={density}
                options={DENSITY_OPTIONS}
                onValueChange={setDensity}
                accessibilityLabel="Interface density"
              />
            }
          />
        </SettingsCard>
      </SettingsSection>
    </View>
  );
}
