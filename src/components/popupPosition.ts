import { useLayoutEffect, useRef, useState } from "react";

export const POPUP_VIEWPORT_MARGIN = 4;

export interface PopupPosition {
  x: number;
  y: number;
}

interface PopupSize {
  width: number;
  height: number;
}

export function getViewportPopupPosition(
  anchor: PopupPosition,
  popup: PopupSize,
  viewport: PopupSize,
  margin = POPUP_VIEWPORT_MARGIN,
): PopupPosition {
  const availableWidth = Math.max(0, viewport.width - margin * 2);
  const availableHeight = Math.max(0, viewport.height - margin * 2);
  const popupWidth = Math.min(Math.max(0, popup.width), availableWidth);
  const popupHeight = Math.min(Math.max(0, popup.height), availableHeight);
  const maxX = Math.max(margin, viewport.width - popupWidth - margin);
  const maxY = Math.max(margin, viewport.height - popupHeight - margin);

  const preferredX =
    anchor.x + popupWidth > viewport.width - margin ? anchor.x - popupWidth : anchor.x;
  const preferredY =
    anchor.y + popupHeight > viewport.height - margin ? anchor.y - popupHeight : anchor.y;

  return {
    x: Math.min(Math.max(preferredX, margin), maxX),
    y: Math.min(Math.max(preferredY, margin), maxY),
  };
}

export function useViewportPopupPosition(anchor: PopupPosition | null, layoutKey?: unknown) {
  const popupRef = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState<PopupPosition | null>(null);

  useLayoutEffect(() => {
    const popup = popupRef.current;
    if (!anchor || !popup) return;

    const updatePosition = () => {
      const next = getViewportPopupPosition(
        anchor,
        { width: popup.offsetWidth, height: popup.offsetHeight },
        { width: window.innerWidth, height: window.innerHeight },
      );

      setPosition((current) => (current?.x === next.x && current.y === next.y ? current : next));
    };

    updatePosition();
    window.addEventListener("resize", updatePosition);

    const resizeObserver =
      typeof ResizeObserver === "undefined" ? null : new ResizeObserver(updatePosition);
    resizeObserver?.observe(popup);

    return () => {
      window.removeEventListener("resize", updatePosition);
      resizeObserver?.disconnect();
    };
  }, [anchor, layoutKey]);

  return {
    popupRef,
    popupPosition: anchor ? (position ?? anchor) : null,
  };
}
