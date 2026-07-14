import { useCallback, useMemo, useRef, useState } from "react";
import { Platform, View } from "react-native";
import { GestureDetector, type GestureType } from "react-native-gesture-handler";

import { useTheme } from "@/styles/use-theme";

interface SidebarResizeHandleProps {
  edge: "left" | "right";
  gesture: GestureType;
  cursorStyle?: object;
}

export function SidebarResizeHandle({ edge, gesture, cursorStyle }: SidebarResizeHandleProps) {
  const theme = useTheme();
  const [active, setActive] = useState(false);
  const hoverTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const highlighted = active;

  const handlePointerEnter = useCallback(() => {
    if (Platform.OS !== "web") return;
    hoverTimerRef.current = setTimeout(() => setActive(true), 150);
  }, []);

  const handlePointerLeave = useCallback(() => {
    if (hoverTimerRef.current) {
      clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
    setActive(false);
  }, []);

  const containerStyle = useMemo(
    () => [
      {
        position: "absolute" as const,
        top: 0,
        bottom: 0,
        width: 10,
        zIndex: 10,
        ...(edge === "right" ? { right: -5 } : { left: -5 }),
      },
      cursorStyle,
    ],
    [cursorStyle, edge],
  );

  const lineStyle = useMemo(
    () => ({
      position: "absolute" as const,
      top: 0,
      bottom: 0,
      width: 1,
      backgroundColor: theme.colors.border,
      ...(edge === "right" ? { right: 4 } : { left: 4 }),
    }),
    [edge, theme.colors.border],
  );

  const accentStyle = useMemo(
    () => ({
      position: "absolute" as const,
      top: 0,
      bottom: 0,
      width: 3,
      backgroundColor: theme.colors.accent,
      ...(edge === "right" ? { right: 3 } : { left: 3 }),
    }),
    [edge, theme.colors.accent],
  );

  return (
    <GestureDetector gesture={gesture}>
      <View
        style={containerStyle}
        onPointerEnter={handlePointerEnter}
        onPointerLeave={handlePointerLeave}
        accessibilityRole="none"
      >
        <View style={[lineStyle, { pointerEvents: "none" }]} />
        {highlighted ? <View style={[accentStyle, { pointerEvents: "none" }]} /> : null}
      </View>
    </GestureDetector>
  );
}
