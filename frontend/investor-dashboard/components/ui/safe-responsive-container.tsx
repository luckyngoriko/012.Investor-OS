"use client";

import { cloneElement, isValidElement, useEffect, useRef, useState } from "react";
import type { PropsWithChildren, ReactElement } from "react";

type SafeResponsiveContainerProps = PropsWithChildren<{
  width?: string | number;
  height?: string | number;
  minReadyWidth?: number;
  minReadyHeight?: number;
  wrapperClassName?: string;
}>;

export function SafeResponsiveContainer({
  children,
  width = "100%",
  height = "100%",
  minReadyWidth = 8,
  minReadyHeight = 8,
  wrapperClassName = "w-full h-full",
}: SafeResponsiveContainerProps) {
  const wrapperRef = useRef<HTMLDivElement | null>(null);
  const [dimensions, setDimensions] = useState({ width: 0, height: 0 });

  useEffect(() => {
    const wrapper = wrapperRef.current;
    if (!wrapper) return;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) return;
      const nextWidth = Math.floor(entry.contentRect.width);
      const nextHeight = Math.floor(entry.contentRect.height);
      setDimensions((current) =>
        current.width === nextWidth && current.height === nextHeight
          ? current
          : { width: nextWidth, height: nextHeight },
      );
    });
    observer.observe(wrapper);

    return () => {
      observer.disconnect();
    };
  }, []);

  const resolvedWidth = typeof width === "number" ? width : dimensions.width;
  const resolvedHeight = typeof height === "number" ? height : dimensions.height;
  const isReady = resolvedWidth >= minReadyWidth && resolvedHeight >= minReadyHeight;

  const chartChild = isValidElement(children)
    ? cloneElement(children as ReactElement<Record<string, unknown>>, {
        width: resolvedWidth,
        height: resolvedHeight,
      })
    : null;

  return (
    <div
      ref={wrapperRef}
      className={wrapperClassName}
      style={{
        ...(typeof width === "number" ? { width: `${width}px` } : {}),
        ...(typeof height === "number" ? { height: `${height}px` } : {}),
      }}
      data-chart-container-ready={isReady ? "true" : "false"}
    >
      {isReady ? chartChild : null}
    </div>
  );
}
