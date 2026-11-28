---
version: alpha
name: jev-curate
description: Design system for the jev-curate marketing site and interactive browser rubric sandbox, built on a light-mode flat editorial aesthetic with TypeSafe brand accents.
colors:
  pink: "#f386a1"
  magenta: "#d45bb6"
  magenta-deep: "#a2278f"
  ink: "oklch(0.14 0.006 60)"
  substrate: "oklch(0.995 0.003 75)"
  surface-muted: "oklch(0.965 0.005 75)"
  grey-1: "oklch(0.26 0.006 60)"
  grey-2: "oklch(0.90 0.006 70)"
  grey-3: "oklch(0.82 0.006 70)"
  grey-4: "oklch(0.77 0.012 180)"
  rule: "color-mix(in oklab, var(--ink) 12%, transparent)"
  bg: "oklch(0.995 0.003 75)"
  fg: "oklch(0.14 0.006 60)"
typography:
  display:
    fontFamily: '"LisaTerminal Paper 2X3Y Medium", "Book Antiqua", Georgia, serif'
  body:
    fontFamily: '"Die Grotesk C Regular", Candara, "Segoe UI", sans-serif'
    fontSize: 18px
    lineHeight: "1.6"
  mono:
    fontFamily: '"JetBrains Mono", "Cascadia Mono", Consolas, monospace'
    fontSize: 13px
rounded:
  base: 4px
spacing:
  space: 5px
  space-5: 20px
  space-8: 40px
---

## Overview

`jev-curate` is a high-throughput dataset sifter for TypeSafe Jev. Its web interface uses a flat, light-mode editorial design ("light Innocent") with high-contrast black ink on a near-white substrate, paired with TypeSafe pink/magenta accents and a monospace terminal aesthetic for CLI, Python, and JSON verdict examples.

## Colors

- Use `{colors.substrate}` as the primary page background (`{colors.bg}`), `{colors.surface-muted}` for recessed surfaces, and `{colors.ink}` as the primary foreground (`{colors.fg}`). Neutral tokens use warm perceptual `oklch()` values with `color-mix(in oklab, ...)` for translucent surfaces and `{colors.rule}` hairlines.
- Enhance `--pink`, `--magenta`, and `--magenta-deep` with `color(display-p3 ...)` inside `@supports (color: color(display-p3 1 1 1))` and `@media (color-gamut: p3)`.
- Use `{colors.grey-1}` for secondary body copy, navigation links, and breadcrumbs.
- Use `{colors.rule}` and `{colors.grey-2}` for hairline borders and section dividers, and `{colors.grey-3}` for decorative separators.
- Use `{colors.magenta-deep}` for small-text accents (such as eyebrows and `sup` badges) and light-surface `:focus-visible` outlines.
- Use `{colors.pink}` for active navigation underlines, scroll-progress indicators, and `:focus-visible` outlines inside dark surfaces (`.terminal`, `.code-block`, `.footer`, `.inline-term`).

## Typography

- Use `{typography.display.fontFamily}` for the wordmark and primary display headings (`h1`, `h2`) with tight negative tracking and balanced wrapping. Set `font-synthesis-weight: none` and `text-rendering: optimizeLegibility` on `html`/`body`, and inherit `font-weight` on `strong` inside headings.
- Use `{typography.body.fontFamily}` for body copy, buttons, and navigation links.
- Use `{typography.mono.fontFamily}` for code blocks, terminal panels, breadcrumbs, keyboard hints (`kbd`), install bars, and toast notifications.

## Layout

- Apply `scrollbar-gutter: stable` on `html` to prevent layout shift across pages.
- Constrain primary content sections (`.hero`, `.breadcrumb`, and content sections) to a centered `1200px` maximum width with generous horizontal padding on desktop, stepping down at `1100px`, `1040px`, `900px`, `768px`, `600px`, and `480px` breakpoints.
- Use a frosted sticky `.site-header` (`color-mix(in oklab, var(--bg) 88%, transparent)` + `backdrop-filter: blur(12px)`) with a `1px` bottom border (`{colors.rule}`) and collapse desktop `.site-nav` into a toggleable `#mobile-nav` drawer below `1040px`.
- Arrange `.reason-list` as a 3-column Bento grid on desktop (first card `span 2`), collapsing to `1fr` below `900px`, and apply `isolation: isolate` on `.terminal` and `.sandbox`.
- Use hairline-only `box-shadow: inset 0 -1px var(--rule)` dividers on `.flag-table` and `.bench-table`, with a sticky first column under `<= 768px`.
- Keep interactive touch targets (`.btn`, `.nav-toggle`, mobile nav links, floating action buttons) at a minimum height of `44px` and apply `touch-action: manipulation` on links, buttons, selects, and tabs.
- Ensure `#sandbox-input` and `#sandbox-preset` maintain at least `16px` font size on mobile (`<= 1040px`) to prevent iOS auto-zoom.

## Elevation & Depth

- Maintain a flat surface hierarchy using `1px` hairline borders (`{colors.rule}`) rather than drop shadows, with subtle `-1px` hover elevation on `.author-links a`, `.ph-launch-banner`, `.steps li`, and `.preset-row` under `@media (hover: hover) and (pointer: fine)`.
- Reserve hard-offset shadows (`--shadow`) and deep elevation shadows exclusively for floating elements such as `.toast` notifications.

## Shapes

- Apply `{rounded.base}` consistently across buttons, banners, install bars, benchmark cards, and keyboard hint pills.
- Use `8px` radius for `.toast` containers.

## Components

- **Hero Install Bar & Trust Line (`.install-bar`, `.trust-line`)**: Single-line `pip` / `cargo` toggle pill with instant copy feedback and a 1-line local `--dry-run` + `secret_scan` trust signal.
- **Before/After Benchmark Card (`.bench-compare`)**: Two-column comparison card in `#cost` contrasting a naive Python LLM-judge loop against `jev-curate` speculative fan-out (`23.9 rows/sec` single node, `444x` cost savings, `0%` parse failures).
- **Buttons (`.btn`)**: Provide `.btn-primary` (solid `{colors.ink}` fill with `{colors.substrate}` text) and `.btn-ghost` (transparent with `{colors.rule}` border and `::before` scale-in hover backdrop). Include a subtle `scale(0.97)` active press state.
- **Terminal & Tabs (`.terminal`, `[role="tablist"]`)**: Support keyboard roving focus (`ArrowLeft`, `ArrowRight`, `Home`, `End`) across `.hero-tabs` and `.sandbox-out-tabs`, with `::before` scale-in hover on unselected `.hero-tab` buttons.
- **Interactive Sandbox (`#sandbox`)**: Pair a preset selector, reset button, and live `secret_scan` status badge with a two-pane input/output grid. Bind the global `/` key to focus `#sandbox-input` (with a pure-CSS `[data-tooltip]` popover on `.kbd-hint`), and flash `.sandbox-out.is-updated` when row counts change.
- **Motion, View Transitions & Print**: Enable cross-document MPA `@view-transition { navigation: auto; }` anchoring `.site-header` and `.logo` with a `180ms` root crossfade. Use `--ease` (`cubic-bezier(0.16, 1, 0.3, 1)`), interruptible `@starting-style` + `.is-leaving` `.toast` transitions, an `80ms` `blur(2px)` copy-swap on `.is-copied`, a `2px` top `.scroll-progress` bar driven by `animation-timeline: scroll()`, and pure-CSS shimmer/border-beam effects wrapped in `@media (prefers-reduced-motion: no-preference)`. Disable all transitions and animations under `@media (prefers-reduced-motion: reduce)`, and provide an ink-saving A4 `@media print` mode that hides chrome and inverts dark surfaces.
