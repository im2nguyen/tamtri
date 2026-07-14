import { useCallback, useEffect, useMemo, useRef } from "react";
import { Platform, useWindowDimensions } from "react-native";
import { Gesture } from "react-native-gesture-handler";
import { runOnJS, useAnimatedStyle, useSharedValue } from "react-native-reanimated";

interface UseResizableSidebarWidthOptions {
  width: number;
  setWidth: (width: number) => void;
  clampWidth: (width: number, viewportWidth?: number) => number;
  edge: "left" | "right";
  enabled?: boolean;
}

export function useResizableSidebarWidth({
  width,
  setWidth,
  clampWidth,
  edge,
  enabled = true,
}: UseResizableSidebarWidthOptions) {
  const { width: viewportWidth } = useWindowDimensions();
  const startWidthRef = useRef(width);
  const resizeWidth = useSharedValue(width);

  useEffect(() => {
    resizeWidth.value = width;
  }, [resizeWidth, width]);

  useEffect(() => {
    if (!enabled) return;
    const clamped = clampWidth(width, viewportWidth);
    if (clamped !== width) {
      setWidth(clamped);
    }
  }, [clampWidth, enabled, setWidth, viewportWidth, width]);

  const persistWidth = useCallback(
    (nextWidth: number) => {
      setWidth(clampWidth(nextWidth, viewportWidth));
    },
    [clampWidth, setWidth, viewportWidth],
  );

  const resizeGesture = useMemo(
    () =>
      Gesture.Pan()
        .enabled(enabled)
        .hitSlop({ left: 8, right: 8, top: 0, bottom: 0 })
        .onStart(() => {
          startWidthRef.current = width;
          resizeWidth.value = width;
        })
        .onUpdate((event) => {
          const delta = edge === "right" ? event.translationX : -event.translationX;
          const nextWidth = clampWidth(startWidthRef.current + delta, viewportWidth);
          resizeWidth.value = nextWidth;
        })
        .onEnd(() => {
          runOnJS(persistWidth)(resizeWidth.value);
        }),
    [clampWidth, edge, enabled, persistWidth, resizeWidth, viewportWidth, width],
  );

  const animatedStyle = useAnimatedStyle(() => ({
    width: resizeWidth.value,
  }));

  const handleCursorStyle =
    Platform.OS === "web" ? ({ cursor: "col-resize" } as object) : undefined;

  return {
    animatedStyle,
    resizeGesture,
    handleCursorStyle,
  };
}
