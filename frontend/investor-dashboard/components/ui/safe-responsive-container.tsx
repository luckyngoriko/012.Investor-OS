"use client";

import { useEffect, useRef, useState } from "react";
import { ResponsiveContainer } from "recharts";
import type { PropsWithChildren } from "react";

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
  const [isReady, setIsReady] = useState(false);

  useEffect(() => {
    const wrapper = wrapperRef.current;
    if (!wrapper) return;

    const updateReadiness = () => {
      const rect = wrapper.getBoundingClientRect();
      setIsReady(rect.width >= minReadyWidth && rect.height >= minReadyHeight);
    };

    updateReadiness();

    const observer = new ResizeObserver(() => updateReadiness());
    observer.observe(wrapper);

    const raf = requestAnimationFrame(updateReadiness);

    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
    };
  }, [minReadyHeight, minReadyWidth]);

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
      {isReady ? (
        <ResponsiveContainer width={width} height={height}>
          {children}
        </ResponsiveContainer>
      ) : null}
    </div>
  );
}
