import { useState } from "react";

interface AvatarProps {
  /** Login / display name — drives the alt text and the initial fallback. */
  login: string;
  src?: string | null;
  size?: number;
  className?: string;
}

function initialOf(login: string): string {
  const trimmed = login.trim();
  return (trimmed[0] ?? "?").toUpperCase();
}

/**
 * A small circular avatar. Renders the image when `src` is present (and loads);
 * otherwise — or on image error — falls back to the login's initial on a
 * monochrome `surface-2` circle. Always hairline-bordered to sit on panels.
 */
export function Avatar({ login, src, size = 24, className = "" }: AvatarProps) {
  const [broken, setBroken] = useState(false);
  const dimension = { width: size, height: size };
  const showImage = !!src && !broken;

  if (showImage) {
    return (
      <img
        src={src as string}
        alt={login}
        width={size}
        height={size}
        loading="lazy"
        onError={() => setBroken(true)}
        style={dimension}
        className={
          "shrink-0 rounded-full border border-border object-cover " + className
        }
      />
    );
  }

  return (
    <span
      aria-label={login}
      role="img"
      style={{ ...dimension, fontSize: Math.max(9, Math.round(size * 0.42)) }}
      className={
        "flex shrink-0 select-none items-center justify-center rounded-full " +
        "border border-border bg-surface-2 font-medium text-text-muted " +
        className
      }
    >
      {initialOf(login)}
    </span>
  );
}

export default Avatar;
