import { Search, X } from "lucide-react-native";
import { forwardRef } from "react";
import {
  Platform,
  Pressable,
  TextInput,
  View,
  type TextInputProps,
  type TextStyle,
  type ViewStyle,
} from "react-native";

import { useTheme } from "@/styles/use-theme";

export interface SearchInputProps extends Omit<TextInputProps, "style"> {
  onClear?: () => void;
  containerStyle?: ViewStyle;
  inputStyle?: TextStyle;
}

export const SearchInput = forwardRef<TextInput, SearchInputProps>(function SearchInput(
  { value, onClear, containerStyle, inputStyle, placeholder = "Search", ...props },
  ref,
) {
  const theme = useTheme();

  return (
    <View
      style={[
        {
          minHeight: 32,
          flexDirection: "row",
          alignItems: "center",
          gap: theme.spacing[2],
          paddingHorizontal: theme.spacing[2],
          borderRadius: theme.radius.lg,
          borderWidth: theme.hairlineWidth,
          borderColor: theme.colors.border,
          backgroundColor: theme.colors.surface2,
        },
        containerStyle,
      ]}
    >
      <Search color={theme.colors.foregroundMuted} size={14} strokeWidth={1.8} />
      <TextInput
        {...props}
        ref={ref}
        value={value}
        placeholder={placeholder}
        placeholderTextColor={theme.colors.foregroundMuted}
        accessibilityLabel={props.accessibilityLabel ?? placeholder}
        style={[
          {
            flex: 1,
            minWidth: 0,
            paddingVertical: theme.spacing[1],
            color: theme.colors.foreground,
            fontFamily: theme.fontFamily.ui,
            fontSize: theme.fontSize.sm,
            ...(Platform.OS === "web" ? { outlineStyle: "none" } : {}),
          } as TextStyle,
          inputStyle,
        ]}
      />
      {value && onClear ? (
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Clear search"
          hitSlop={8}
          onPress={onClear}
          style={({ pressed }) => ({ opacity: pressed ? 0.55 : 0.8 })}
        >
          <X color={theme.colors.foregroundMuted} size={14} strokeWidth={1.8} />
        </Pressable>
      ) : null}
    </View>
  );
});
