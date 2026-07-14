import { Monitor, Moon, Sun } from "lucide-react-native";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Platform, Text, TextInput, View, type TextStyle } from "react-native";

import { SettingsSection, useSettingsStyles } from "@/components/settings/settings-section";
import { DropdownSelect } from "@/components/ui/dropdown-select";
import {
  parseClampedFontSize,
  sanitizeFontFamily,
  useAppearanceStore,
  type ThemeMode,
} from "@/stores/appearance-store";
import {
  DEFAULT_MONO_FONT_STACK,
  DEFAULT_UI_FONT_STACK,
  MAX_CODE_FONT_SIZE,
  MAX_UI_FONT_SIZE,
  MIN_CODE_FONT_SIZE,
  MIN_UI_FONT_SIZE,
} from "@/styles/tokens";
import { SYNTAX_THEME_OPTIONS, type SyntaxThemeId } from "@/styles/syntax-themes";
import { useTheme } from "@/styles/use-theme";
import { AppearancePreview } from "@/screens/settings/appearance-preview";

function sizeDraftToOverride(value: string): number | undefined {
  if (value.length === 0) return undefined;
  const parsed = Number.parseInt(value, 10);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function ThemeIcon({ mode, color }: { mode: ThemeMode; color: string }) {
  const theme = useTheme();
  const size = 16;
  if (mode === "light") return <Sun color={color} size={size} />;
  if (mode === "dark") return <Moon color={color} size={size} />;
  return <Monitor color={color} size={size} />;
}

interface FontFamilyRowProps {
  title: string;
  hint: string;
  accessibilityLabel: string;
  placeholder: string;
  value: string;
  draft: string;
  withBorder: boolean;
  onChangeDraft: (value: string) => void;
  onCommit: (value: string) => void;
}

function FontFamilyRow({
  title,
  hint,
  accessibilityLabel,
  placeholder,
  value,
  draft,
  withBorder,
  onChangeDraft,
  onCommit,
}: FontFamilyRowProps) {
  const settingsStyles = useSettingsStyles();
  const theme = useTheme();

  useEffect(() => {
    onChangeDraft(value);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value]);

  return (
    <View style={withBorder ? settingsStyles.rowWithBorder : settingsStyles.row}>
      <View style={settingsStyles.rowContent}>
        <Text style={settingsStyles.rowTitle}>{title}</Text>
        <Text style={settingsStyles.rowHint}>{hint}</Text>
      </View>
      <TextInput
        value={draft}
        onChangeText={onChangeDraft}
        onBlur={() => onCommit(draft)}
        onSubmitEditing={() => onCommit(draft)}
        placeholder={placeholder}
        placeholderTextColor={theme.colors.foregroundMuted}
        autoCapitalize="none"
        autoCorrect={false}
        spellCheck={false}
        style={
          {
            flexGrow: 1,
            flexShrink: 1,
            maxWidth: 280,
            minHeight: 36,
            paddingVertical: theme.spacing[2],
            paddingHorizontal: theme.spacing[3],
            borderRadius: theme.radius.md,
            borderWidth: 1,
            borderColor: theme.colors.border,
            backgroundColor: theme.colors.surface2,
            color: theme.colors.foreground,
            fontSize: theme.fontSize.sm,
            textAlign: "left",
            ...(Platform.OS === "web" ? { outlineStyle: "none" } : {}),
          } as TextStyle
        }
        accessibilityLabel={accessibilityLabel}
      />
    </View>
  );
}

interface FontSizeRowProps {
  title: string;
  accessibilityLabel: string;
  draft: string;
  withBorder?: boolean;
  onChangeDraft: (value: string) => void;
  onCommit: () => void;
}

function FontSizeRow({
  title,
  accessibilityLabel,
  draft,
  withBorder = true,
  onChangeDraft,
  onCommit,
}: FontSizeRowProps) {
  const settingsStyles = useSettingsStyles();
  const theme = useTheme();

  return (
    <View style={withBorder ? settingsStyles.rowWithBorder : settingsStyles.row}>
      <View style={settingsStyles.rowContent}>
        <Text style={settingsStyles.rowTitle}>{title}</Text>
      </View>
      <View style={{ flexDirection: "row", alignItems: "center", gap: theme.spacing[2] }}>
        <TextInput
          value={draft}
          onChangeText={onChangeDraft}
          onBlur={onCommit}
          onSubmitEditing={onCommit}
          keyboardType="number-pad"
          inputMode="numeric"
          selectTextOnFocus
          style={
            {
              width: 64,
              minHeight: 36,
              paddingVertical: theme.spacing[2],
              paddingHorizontal: theme.spacing[3],
              borderRadius: theme.radius.md,
              borderWidth: 1,
              borderColor: theme.colors.border,
              backgroundColor: theme.colors.surface2,
              color: theme.colors.foreground,
              fontSize: theme.fontSize.sm,
              textAlign: "right",
              ...(Platform.OS === "web" ? { outlineStyle: "none" } : {}),
            } as TextStyle
          }
          accessibilityLabel={accessibilityLabel}
        />
        <Text style={{ color: theme.colors.foregroundMuted, fontSize: theme.fontSize.sm }}>px</Text>
      </View>
    </View>
  );
}

const THEME_OPTIONS: Array<{ value: ThemeMode; label: string }> = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

export function AppearanceSection() {
  const theme = useTheme();
  const settingsStyles = useSettingsStyles();
  const showFontFamilyRows = Platform.OS === "web";

  const themeMode = useAppearanceStore((s) => s.themeMode);
  const uiFontFamily = useAppearanceStore((s) => s.uiFontFamily);
  const uiFontSize = useAppearanceStore((s) => s.uiFontSize);
  const monoFontFamily = useAppearanceStore((s) => s.monoFontFamily);
  const codeFontSize = useAppearanceStore((s) => s.codeFontSize);
  const syntaxTheme = useAppearanceStore((s) => s.syntaxTheme);
  const setThemeMode = useAppearanceStore((s) => s.setThemeMode);
  const setUiFontFamily = useAppearanceStore((s) => s.setUiFontFamily);
  const setUiFontSize = useAppearanceStore((s) => s.setUiFontSize);
  const setMonoFontFamily = useAppearanceStore((s) => s.setMonoFontFamily);
  const setCodeFontSize = useAppearanceStore((s) => s.setCodeFontSize);
  const setSyntaxTheme = useAppearanceStore((s) => s.setSyntaxTheme);

  const [uiFontDraft, setUiFontDraft] = useState(uiFontFamily);
  const [monoFontDraft, setMonoFontDraft] = useState(monoFontFamily);
  const [uiSizeDraft, setUiSizeDraft] = useState(String(uiFontSize));
  const [codeSizeDraft, setCodeSizeDraft] = useState(String(codeFontSize));

  useEffect(() => setUiSizeDraft(String(uiFontSize)), [uiFontSize]);
  useEffect(() => setCodeSizeDraft(String(codeFontSize)), [codeFontSize]);

  const themeDropdownOptions = useMemo(
    () =>
      THEME_OPTIONS.map((option) => ({
        ...option,
        leading: <ThemeIcon mode={option.value} color={theme.colors.foregroundMuted} />,
      })),
    [theme.colors.foregroundMuted],
  );

  const syntaxOptions = useMemo(
    () =>
      SYNTAX_THEME_OPTIONS.map((option) => ({
        value: option.id,
        label: option.label,
      })),
    [],
  );

  const commitUiFontFamily = useCallback(
    (value: string) => {
      const sanitized = sanitizeFontFamily(value);
      if (sanitized === null) {
        setUiFontDraft(uiFontFamily);
        return;
      }
      setUiFontDraft(sanitized);
      if (sanitized !== uiFontFamily) setUiFontFamily(sanitized);
    },
    [setUiFontFamily, uiFontFamily],
  );

  const commitMonoFontFamily = useCallback(
    (value: string) => {
      const sanitized = sanitizeFontFamily(value);
      if (sanitized === null) {
        setMonoFontDraft(monoFontFamily);
        return;
      }
      setMonoFontDraft(sanitized);
      if (sanitized !== monoFontFamily) setMonoFontFamily(sanitized);
    },
    [monoFontFamily, setMonoFontFamily],
  );

  const commitUiSize = useCallback(() => {
    const parsed = parseClampedFontSize(uiSizeDraft, { min: MIN_UI_FONT_SIZE, max: MAX_UI_FONT_SIZE });
    const next = parsed ?? uiFontSize;
    setUiSizeDraft(String(next));
    if (next !== uiFontSize) setUiFontSize(next);
  }, [setUiFontSize, uiFontSize, uiSizeDraft]);

  const commitCodeSize = useCallback(() => {
    const parsed = parseClampedFontSize(codeSizeDraft, {
      min: MIN_CODE_FONT_SIZE,
      max: MAX_CODE_FONT_SIZE,
    });
    const next = parsed ?? codeFontSize;
    setCodeSizeDraft(String(next));
    if (next !== codeFontSize) setCodeFontSize(next);
  }, [codeFontSize, codeSizeDraft, setCodeFontSize]);

  const previewOverrides = useMemo(
    () => ({
      monoFontFamily: monoFontDraft,
      codeFontSize: sizeDraftToOverride(codeSizeDraft),
    }),
    [codeSizeDraft, monoFontDraft],
  );

  return (
    <View>
      <SettingsSection title="Theme">
        <View style={settingsStyles.card}>
          <View style={settingsStyles.row}>
            <View style={settingsStyles.rowContent}>
              <Text style={settingsStyles.rowTitle}>Appearance</Text>
            </View>
            <DropdownSelect
              value={themeMode}
              options={themeDropdownOptions}
              onChange={setThemeMode}
              accessibilityLabel={`Theme: ${themeMode}`}
            />
          </View>
        </View>
      </SettingsSection>

      <SettingsSection title="Fonts">
        <View style={settingsStyles.card}>
          {showFontFamilyRows ? (
            <FontFamilyRow
              title="Interface font"
              hint="CSS font-family stack"
              accessibilityLabel="Interface font family"
              placeholder={DEFAULT_UI_FONT_STACK}
              value={uiFontFamily}
              draft={uiFontDraft}
              withBorder={false}
              onChangeDraft={setUiFontDraft}
              onCommit={commitUiFontFamily}
            />
          ) : null}
          <FontSizeRow
            title="Interface size"
            accessibilityLabel="Interface font size"
            draft={uiSizeDraft}
            withBorder={showFontFamilyRows}
            onChangeDraft={(value) => setUiSizeDraft(value.replace(/[^\d]/g, ""))}
            onCommit={commitUiSize}
          />
          {showFontFamilyRows ? (
            <FontFamilyRow
              title="Code font"
              hint="Monospace stack for diffs and code"
              accessibilityLabel="Code font family"
              placeholder={DEFAULT_MONO_FONT_STACK}
              value={monoFontFamily}
              draft={monoFontDraft}
              withBorder
              onChangeDraft={setMonoFontDraft}
              onCommit={commitMonoFontFamily}
            />
          ) : null}
          <FontSizeRow
            title="Code size"
            accessibilityLabel="Code font size"
            draft={codeSizeDraft}
            onChangeDraft={(value) => setCodeSizeDraft(value.replace(/[^\d]/g, ""))}
            onCommit={commitCodeSize}
          />
        </View>
      </SettingsSection>

      <SettingsSection title="Syntax">
        <View style={settingsStyles.card}>
          <View style={settingsStyles.row}>
            <View style={settingsStyles.rowContent}>
              <Text style={settingsStyles.rowTitle}>Highlight theme</Text>
              <Text style={settingsStyles.rowHint}>Independent of app light/dark mode</Text>
            </View>
            <DropdownSelect
              value={syntaxTheme}
              options={syntaxOptions}
              onChange={(value: SyntaxThemeId) => setSyntaxTheme(value)}
              accessibilityLabel={`Syntax theme: ${syntaxTheme}`}
            />
          </View>
        </View>
        <View style={{ marginTop: theme.spacing[4] }}>
          <AppearancePreview overrides={previewOverrides} />
        </View>
      </SettingsSection>
    </View>
  );
}
