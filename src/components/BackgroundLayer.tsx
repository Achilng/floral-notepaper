import { convertFileSrc } from "@tauri-apps/api/core";
import { useMemo } from "react";
import type { AppConfig } from "../features/settings/types";

interface BackgroundLayerProps {
  config: AppConfig | null;
}

export function BackgroundLayer({ config }: BackgroundLayerProps) {
  const rawPath = config?.backgroundImagePath?.trim() ?? "";
  const convertedUrl = useMemo(() => (rawPath ? convertFileSrc(rawPath) : ""), [rawPath]);

  if (!rawPath) return null;

  const fit = config?.backgroundFit ?? "cover";
  const dim = Math.max(0, Math.min(1, config?.backgroundDim ?? 0.25));
  const blur = Math.max(0, Math.min(20, config?.backgroundBlur ?? 0));
  const scale = Math.max(0.5, Math.min(2, config?.backgroundScale ?? 1));
  const positionX = Math.max(0, Math.min(100, config?.backgroundPositionX ?? 50));
  const positionY = Math.max(0, Math.min(100, config?.backgroundPositionY ?? 50));

  const imageStyle = {
    objectPosition: `${positionX}% ${positionY}%` as const,
    filter: blur > 0 ? `blur(${blur}px)` : undefined,
    // CSS blur samples beyond image edges, causing pale fringes. No clean fix yet.
    transform: `scale(${scale})`,
    transformOrigin: `${positionX}% ${positionY}%`,
  };

  return (
    <div className="absolute inset-0 pointer-events-none overflow-hidden z-0">
      {fit === "repeat" ? (
        <div
          className="absolute inset-0"
          style={{
            backgroundImage: `url("${convertedUrl}")`,
            backgroundSize: "auto",
            backgroundPosition: `${positionX}% ${positionY}%`,
            backgroundRepeat: "repeat",
            ...imageStyle,
          }}
        />
      ) : (
        <img
          src={convertedUrl}
          alt=""
          className="absolute inset-0 w-full h-full"
          style={{
            objectFit: fit === "contain" ? "contain" : "cover",
            ...imageStyle,
          }}
        />
      )}
      <div className="absolute inset-0 bg-cloud" style={{ opacity: dim }} />
    </div>
  );
}
