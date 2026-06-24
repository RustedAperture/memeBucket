import * as React from "react";

interface UseTouchHoldProps {
  onTap: () => void;
  onLongPress: () => void;
  delay?: number;
}

export function useTouchHold({ onTap, onLongPress, delay = 500 }: UseTouchHoldProps) {
  const timeoutRef = React.useRef<NodeJS.Timeout | null>(null);
  const hasMovedRef = React.useRef(false);
  const isLongPressRef = React.useRef(false);
  const startCoordsRef = React.useRef({ x: 0, y: 0 });

  const onTouchStart = React.useCallback((e: React.TouchEvent) => {
    if (e.touches.length > 1) return;

    const touch = e.touches[0];
    startCoordsRef.current = { x: touch.clientX, y: touch.clientY };
    hasMovedRef.current = false;
    isLongPressRef.current = false;

    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }

    timeoutRef.current = setTimeout(() => {
      isLongPressRef.current = true;
      onLongPress();
    }, delay);
  }, [onLongPress, delay]);

  const onTouchEnd = React.useCallback((e: React.TouchEvent) => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }

    if (!isLongPressRef.current && !hasMovedRef.current) {
      e.preventDefault();
      onTap();
    }
  }, [onTap]);

  const onTouchMove = React.useCallback((e: React.TouchEvent) => {
    if (e.touches.length > 1) return;

    const touch = e.touches[0];
    const dx = touch.clientX - startCoordsRef.current.x;
    const dy = touch.clientY - startCoordsRef.current.y;

    if (Math.abs(dx) > 10 || Math.abs(dy) > 10) {
      hasMovedRef.current = true;
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
        timeoutRef.current = null;
      }
    }
  }, []);

  const onTouchCancel = React.useCallback(() => {
    hasMovedRef.current = true;
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
  }, []);

  return {
    onTouchStart,
    onTouchEnd,
    onTouchMove,
    onTouchCancel,
  };
}
