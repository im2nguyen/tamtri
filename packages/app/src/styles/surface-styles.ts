import { Platform, type ViewStyle } from "react-native";

import type { AppTheme } from "./build-theme";

/** Raised content card chrome. Apply at the sidebar/content boundary only. */
export function raisedContentCardStyle(theme: AppTheme): ViewStyle {
  return {
    backgroundColor: theme.colors.surfaceWorkspace,
    borderTopLeftRadius: theme.radius.contentCard,
    borderBottomLeftRadius: theme.radius.contentCard,
    borderLeftWidth: theme.hairlineWidth,
    borderLeftColor: theme.colors.contentSeam,
    ...(Platform.OS === "web"
      ? ({ boxShadow: "-6px 0 12px -10px var(--tamtri-shadow)" } as unknown as ViewStyle)
      : {
          shadowColor: theme.colors.shadow,
          shadowOffset: { width: -4, height: 0 },
          shadowOpacity: 0.3,
          shadowRadius: 8,
        }),
  };
}

/** Frosted menu/popover chrome. Native callers get the same translucent fill without blur. */
export function frostedPopupStyle(theme: AppTheme): ViewStyle {
  return {
    overflow: "hidden",
    borderRadius: theme.radius.xl,
    borderWidth: theme.hairlineWidth,
    borderColor: theme.colors.popupBorder,
    backgroundColor: theme.colors.popupSurface,
    ...(Platform.OS === "web"
      ? ({
          backdropFilter: "blur(24px) saturate(150%)",
          boxShadow: "0 10px 32px -14px var(--tamtri-shadow)",
        } as unknown as ViewStyle)
      : {
          shadowColor: theme.colors.shadow,
          shadowOffset: { width: 0, height: 8 },
          shadowOpacity: 0.24,
          shadowRadius: 16,
          elevation: 8,
        }),
  };
}
