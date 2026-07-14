import { Platform, View, type ViewProps } from "react-native";

import { isDesktopHost } from "@/constants/layout";

/** Electron frameless window drag region (VS Code style). */
export function TitlebarDragRegion({ style, ...props }: ViewProps) {
  if (!isDesktopHost() || Platform.OS !== "web") return null;
  return (
    <View
      {...props}
      style={[
        {
          position: "absolute",
          top: 0,
          left: 72,
          right: 0,
          height: 38,
          zIndex: 1,
          // @ts-expect-error web-only
          WebkitAppRegion: "drag",
        },
        style,
      ]}
    />
  );
}
