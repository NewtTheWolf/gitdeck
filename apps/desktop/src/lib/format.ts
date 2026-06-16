/**
 * Compact a count into a short label, e.g. 1234 → "1.2k", 1500000 → "1.5M".
 * Small values (< 1000) are returned as-is. Negative / non-finite inputs
 * collapse to "0" so the UI never shows "NaN".
 */
export function formatStars(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return "0";
  if (value < 1000) return String(Math.floor(value));
  if (value < 1_000_000) {
    const k = value / 1000;
    return `${trim(k)}k`;
  }
  const m = value / 1_000_000;
  return `${trim(m)}M`;
}

function trim(n: number): string {
  // One decimal place, but drop a trailing ".0" (1.0k → 1k).
  const rounded = Math.round(n * 10) / 10;
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
}

/**
 * A small palette of GitHub language colors. Unknown languages fall back to a
 * neutral muted dot so the layout stays consistent.
 */
const LANGUAGE_COLORS: Record<string, string> = {
  JavaScript: "#f1e05a",
  TypeScript: "#3178c6",
  Python: "#3572A5",
  Rust: "#dea584",
  Go: "#00ADD8",
  Java: "#b07219",
  C: "#555555",
  "C++": "#f34b7d",
  "C#": "#178600",
  Ruby: "#701516",
  PHP: "#4F5D95",
  Swift: "#F05138",
  Kotlin: "#A97BFF",
  Dart: "#00B4AB",
  Shell: "#89e051",
  HTML: "#e34c26",
  CSS: "#563d7c",
  Vue: "#41b883",
  Svelte: "#ff3e00",
  Lua: "#000080",
  Elixir: "#6e4a7e",
  Haskell: "#5e5086",
  Scala: "#c22d40",
  Zig: "#ec915c",
  Nix: "#7e7eff",
};

export function languageColor(language: string | null | undefined): string {
  if (!language) return "#8793a8";
  return LANGUAGE_COLORS[language] ?? "#8793a8";
}

export interface LabelChipStyle {
  /** Solid color for a small dot. */
  dot: string;
  /** Very subtle tinted background. */
  background: string;
  /** Hairline border, slightly stronger than the background. */
  borderColor: string;
}

/**
 * Turn a GitHub label color (a 6-digit hex, no leading "#") into a subtle
 * tinted-chip style: a solid dot plus a low-alpha background and border. Per our
 * design rules label colors are accents only — never a filled background — so
 * the chip text stays in the normal text color. Invalid/empty colors return a
 * neutral monochrome chip. Adapted from gitdeck's utils/colors.ts.
 */
export function labelChipStyle(hex: string | null | undefined): LabelChipStyle {
  const cleaned = (hex || "").replace("#", "").trim();
  if (cleaned.length < 6) {
    return {
      dot: "rgba(255,255,255,0.32)",
      background: "rgba(255,255,255,0.04)",
      borderColor: "rgba(255,255,255,0.08)",
    };
  }
  const r = Number.parseInt(cleaned.slice(0, 2), 16);
  const g = Number.parseInt(cleaned.slice(2, 4), 16);
  const b = Number.parseInt(cleaned.slice(4, 6), 16);
  if ([r, g, b].some((v) => Number.isNaN(v))) {
    return {
      dot: "rgba(255,255,255,0.32)",
      background: "rgba(255,255,255,0.04)",
      borderColor: "rgba(255,255,255,0.08)",
    };
  }
  return {
    dot: `rgb(${r} ${g} ${b})`,
    background: `rgba(${r}, ${g}, ${b}, 0.1)`,
    borderColor: `rgba(${r}, ${g}, ${b}, 0.28)`,
  };
}

export type RelativeTimeLang = "en" | "de";

const RELATIVE_TIME_LABELS: Record<
  RelativeTimeLang,
  {
    now: string;
    minute: (n: number) => string;
    hour: (n: number) => string;
    day: (n: number) => string;
    month: (n: number) => string;
    year: (n: number) => string;
  }
> = {
  en: {
    now: "just now",
    minute: (n) => `${n}m ago`,
    hour: (n) => `${n}h ago`,
    day: (n) => `${n}d ago`,
    month: (n) => `${n}mo ago`,
    year: (n) => `${n}y ago`,
  },
  de: {
    now: "gerade eben",
    minute: (n) => `vor ${n} Min.`,
    hour: (n) => `vor ${n} Std.`,
    day: (n) => `vor ${n} T`,
    month: (n) => `vor ${n} Mon.`,
    year: (n) => `vor ${n} J`,
  },
};

/**
 * Format an ISO timestamp as a compact relative string ("3h ago"). Ported from
 * gitdeck's utils/format.ts, trimmed to the locales this app ships (en/de).
 * Unknown languages and invalid dates degrade to "" / English.
 */
export function formatRelativeTime(
  iso: string,
  now = Date.now(),
  language: string = "en",
): string {
  if (!iso) return "";
  const ms = new Date(iso).getTime();
  if (!Number.isFinite(ms)) return "";
  const labels =
    RELATIVE_TIME_LABELS[(language as RelativeTimeLang) in RELATIVE_TIME_LABELS
      ? (language as RelativeTimeLang)
      : "en"];
  const minutes = Math.floor((now - ms) / 60_000);
  if (minutes < 1) return labels.now;
  if (minutes < 60) return labels.minute(minutes);
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return labels.hour(hours);
  const days = Math.floor(hours / 24);
  if (days < 30) return labels.day(days);
  const months = Math.floor(days / 30);
  if (months < 12) return labels.month(months);
  return labels.year(Math.floor(months / 12));
}
