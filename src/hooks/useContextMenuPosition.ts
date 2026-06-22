import { useLayoutEffect, useRef, useState } from 'react';

const MARGIN = 8;

/**
 * Position a fixed context menu so it never overflows the viewport.
 * Uses the real rendered dimensions, so long menus (e.g. feeds with many
 * folders) are repositioned above the cursor instead of being clipped.
 *
 * Pass any values that change the menu's content (modal state, list length,
 * etc.) in `deps` so the position is recalculated when the menu grows/shrinks.
 */
export function useContextMenuPosition(x: number, y: number, deps: React.DependencyList = []) {
  const ref = useRef<HTMLDivElement>(null);
  const [style, setStyle] = useState<{ left: number; top: number; maxHeight?: number; overflowY?: 'auto' }>({ left: x, top: y });

  useLayoutEffect(() => {
    if (!ref.current) return;
    const rect = ref.current.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;

    let left = x;
    let top = y;

    if (left + rect.width > vw - MARGIN) {
      left = vw - rect.width - MARGIN;
    }
    if (left < MARGIN) {
      left = MARGIN;
    }
    if (top + rect.height > vh - MARGIN) {
      top = vh - rect.height - MARGIN;
    }
    if (top < MARGIN) {
      top = MARGIN;
    }

    setStyle({ left, top, maxHeight: vh - MARGIN * 2, overflowY: 'auto' });
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [x, y, ...deps]);

  return { ref, style };
}
