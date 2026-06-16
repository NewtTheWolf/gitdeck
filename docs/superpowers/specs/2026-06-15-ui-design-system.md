# Newt Todo — UI Design System ("Refined developer instrument")

Derived from an adversarially-verified deep-research report (`/loop` deep-research, run wf_8b233b51) on what makes minimal premium dev-tool UIs (Linear/Vercel/Raycast) feel crafted, not "AI slop". Direction chosen by the user: **minimal & sharp**.

## Aesthetic
Quiet monochrome dark canvas, ONE rationed indigo accent (<15% of the UI), hairline-bordered surfaces (structure from 1px borders, not floating shadows), Geist Sans + Geist Mono (mono for machine data), Lucide line icons, tabular numerals, compositor-only motion, keyboard-first focus rings. Sharp, restrained, premium.

## Verified rules (apply as hard constraints)
- **One accent, <15% of UI.** Reserve indigo for: active nav item, primary button, focus ring, and small status/unread dots. Cards, sidebar, todo rows stay **fully monochrome**. PR/issue state shows as a **small colored dot/badge only**, never a filled background.
- **Hairline borders carry structure.** 1px `rgba(255,255,255,.08)` default, `.14` strong (hover/active). Every card/input/divider gets a hairline. No heavy single drop shadows on flat cards.
- **Elevation only for overlays.** Stacked shadow + inset hairline ring, ONLY for dropdowns/menus/modals (account switcher menu, command palette later). Flat cards rely on border + surface step.
- **Concentric radii.** child radius ≤ parent; inner = outer − padding. Radii 4/6/8 + pill (9999) for status chips.
- **Type:** scale 12/13/14/16/18/24/32; line-height ~1.15 headings / ~1.45 body; weights ≤600 (use 400 body, 500 default-UI, 600 emphasis/active). Size-dependent tight tracking on big text. **`tabular-nums`** on every number (stars, forks, PR/issue numbers, counts) — Geist Mono gives this naturally.
- **Motion:** never `transition: all`. Animate only `opacity, transform, background-color, border-color, color`. Hover 120–160ms ease-out; larger surfaces 200–250ms. Respect `prefers-reduced-motion`.
- **A11y:** `:focus-visible` ring on every focusable element (`ring-2 ring-accent ring-offset-2 ring-offset-canvas`), reset default `:focus` outline. `color-scheme: dark` on `<html>`.
- **Anti-slop:** NO body/background gradients, NO glassmorphism, NO emoji/unicode glyphs in chrome (use Lucide SVGs), NO centered-everything (left-align content, sidebar-led layout), consistent spacing, monochrome cards.

## Tailwind v4 `@theme` tokens (target `src/app.css`)
```css
@import "tailwindcss";
@theme {
  /* surfaces — Linear-style dark ramp */
  --color-canvas:     #08090a;
  --color-surface:    #0f1011;   /* sidebar, panels */
  --color-surface-2:  #161718;   /* raised cards / inputs */
  --color-surface-3:  #1d1e20;   /* hover */
  /* hairline borders (translucent) */
  --color-border:        rgba(255,255,255,0.08);
  --color-border-strong: rgba(255,255,255,0.14);
  /* text */
  --color-text:        #f7f8f8;
  --color-text-muted:  #8a8f98;  /* secondary */
  --color-text-faint:  #62666d;  /* tertiary / disabled */
  /* the single accent: indigo */
  --color-accent:        #5e6ad2;
  --color-accent-hover:  #6b76e0;
  --color-accent-fg:     #ffffff;
  /* semantic — DOTS/BADGES ONLY, never fills */
  --color-open:    #3fb950;  /* issue/PR open */
  --color-merged:  #a371f7;  /* PR merged */
  --color-closed:  #f85149;  /* closed */
  --color-warn:    #d29922;
  /* radii */
  --radius-sm: 4px;
  --radius:    6px;
  --radius-lg: 8px;
  /* fonts */
  --font-sans: "Geist Variable", ui-sans-serif, system-ui, sans-serif;
  --font-mono: "Geist Mono Variable", ui-monospace, "SF Mono", monospace;
  /* overlay elevation only */
  --shadow-overlay: 0 0 0 1px rgba(255,255,255,0.06), 0 2px 8px rgba(0,0,0,0.32), 0 12px 32px rgba(0,0,0,0.44);
}
@layer base {
  html { color-scheme: dark; }
  body { background: var(--color-canvas); color: var(--color-text);
         font-family: var(--font-sans);
         -webkit-font-smoothing: antialiased; text-rendering: optimizeLegibility; }
  .tnum { font-variant-numeric: tabular-nums; }
}
```

## Fonts (offline-safe, self-hosted)
`@fontsource-variable/geist` + `@fontsource-variable/geist-mono` (Bun add); import both in `main.tsx`. Geist Mono for: repo `owner/repo` paths, all numbers/counts, PR/issue numbers, code/metadata. Geist Sans for everything else.

## Icons
`lucide-react`. Replace ALL unicode glyphs (★ ⊙ etc.). Sizing: 14–16px inline with text, 16–18px for nav, `stroke-width: 1.75`, `text-muted` by default, `text-text`/`text-accent` on active/hover. Optical-align with the text baseline.

## Component guidance
- **Shell:** narrow left sidebar (~240px) on `--surface`, hairline right border. Sidebar = app mark + nav (Lucide icons + label, active item: accent text + a 2px accent left-rail or subtle `--surface-2` pill) + account switcher (bottom) + locale/menu. Main = `--canvas`, content max-width for readability, NOT centered card-in-void.
- **Lists/rows (todos, issues):** row height ~36–40px, hairline divider or gap, hover `--surface-2`, left-aligned, monochrome; status as a leading dot. Checkboxes: custom, accent when checked.
- **Cards (repos):** `--surface` bg, hairline border, hover → `--surface-2` + `--border-strong` (no shadow growth, optional `translateY(-1px)`). Title in Geist Mono (`owner/repo`), muted description (line-clamp-2), a footer row of mono stats with Lucide icons (star/issue/fork) + a language dot. Concentric inner radii.
- **Buttons:** primary = accent bg / accent-fg, hover `--accent-hover`; secondary = `--surface-2` + hairline, hover `--border-strong`. 6px radius, 500 weight, focus-visible ring.
- **Inputs:** `--surface-2`, hairline, focus → accent border + ring. No heavy glow.
- **Empty states:** a Lucide icon (muted), one line of sans copy, a single accent action. Left-aligned in the panel, not a giant centered hero.
- **Loading:** subtle skeletons — `--surface-2` blocks at low opacity with a slow opacity pulse (no rainbow shimmer). Match the real layout's shape.

## Scope of the redesign
Apply to: `app.css` (@theme + base + fonts), `main.tsx` (font imports), `components/Shell.tsx`, `pages/Tasks.tsx`, `pages/Settings.tsx`, `pages/Dashboard.tsx`, `components/dashboard/RepoGrid.tsx`, a new `components/ui/Icon`-usage via lucide, and shared primitives if helpful (`components/ui/Button.tsx`, `Badge.tsx`, `Skeleton.tsx`). Keep ALL behavior + i18n + the Tauri command wiring identical.
