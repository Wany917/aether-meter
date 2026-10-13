import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { type RefObject, useEffect } from "react";

/**
 * Keep the window exactly as tall as `ref`'s content: an overlay with empty
 * transparent space would still catch clicks meant for the game.
 */
export function useFitWindowHeight(ref: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const window = getCurrentWindow();
    let last = 0;
    const observer = new ResizeObserver(([entry]) => {
      const height = Math.ceil(entry?.borderBoxSize[0]?.blockSize ?? element.offsetHeight);
      if (height > 0 && height !== last) {
        last = height;
        void window.setSize(new LogicalSize(document.documentElement.clientWidth, height));
      }
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
}
