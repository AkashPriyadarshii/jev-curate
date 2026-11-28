---
version: alpha
name: jev-curate
description: Design system for the jev-curate marketing site and interactive browser rubric sandbox, built on a light-mode flat editorial aesthetic with TypeSafe brand accents.
colors:
  pink: "#f386a1"
  magenta: "#d45bb6"
  magenta-deep: "#a2278f"
  ink: "#000000"
  substrate: "#fefefe"
  grey-1: "#1e1e1e"
  grey-2: "#dedede"
  grey-3: "#c4c4c4"
  grey-4: "#abbab9"
  bg: "#fefefe"
  fg: "#000000"
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

- Use `{colors.substrate}` as the primary page background (`{colors.bg}`) and `{colors.ink}` as the primary foreground (`{colors.fg}`).
- Use `{colors.grey-1}` for secondary body copy, navigation links, and breadcrumbs.
- Use `{colors.grey-2}` for hairline borders and section dividers, and `{colors.grey-3}` for decorative separators.
- Use `{colors.magenta-deep}` for small-text accents (such as eyebrows and `sup` badges) to maintain AA contrast on light backgrounds.
- Use `{colors.pink}` for active navigation underlines and scroll-progress indicators, and `{colors.magenta}` for `:focus-visible` outlines.

## Typography

- Use `{typography.display.fontFamily}` for the wordmark and primary display headings (`h1`, `h2`) with tight negative tracking and balanced wrapping.
- Use `{typography.body.fontFamily}` for body copy, buttons, and navigation links.
- Use `{typography.mono.fontFamily}` for code blocks, terminal panels, breadcrumbs, keyboard hints (`kbd`), and toast notifications.

## Layout

- Constrain primary content sections (`.hero`, `.breadcrumb`, and content sections) to a centered `1200px` maximum width with generous horizontal padding on desktop, stepping down at `1100px`, `1040px`, `600px`, and `480px` breakpoints.
- Use a sticky `.site-header` with `1px` bottom border (`{colors.grey-2}`) and collapse desktop `.site-nav` into a toggleable `#mobile-nav` drawer below `1040px`.
- Keep interactive touch targets (`.btn`, `.nav-toggle`, mobile nav links, floating action buttons) at a minimum height of `44px` and apply `touch-action: manipulation` on links, buttons, selects, and tabs.
- Ensure `#sandbox-input` and `#sandbox-preset` maintain at least `16px` font size on mobile (`<= 1040px`) to prevent iOS auto-zoom.

## Elevation & Depth

- Maintain a flat surface hierarchy using `1px` hairline borders (`{colors.grey-2}` or `rgba(0, 0, 0, 0.12)`) rather than drop shadows.
- Reserve hard-offset shadows (`--shadow`) and deep elevation shadows exclusively for floating elements such as `.toast` notifications.

## Shapes

- Apply `{rounded.base}` consistently across buttons, banners, and keyboard hint pills.
- Use `8px` radius for `.toast` containers.

## Components

- **Buttons (`.btn`)**: Provide `.btn-primary` (solid `{colors.ink}` fill with `{colors.substrate}` text) and `.btn-ghost` (transparent with `rgba(0, 0, 0, 0.12)` border). Include a subtle `scale(0.97)` active press state.
- **Terminal & Tabs (`.terminal`, `[role="tablist"]`)**: Support keyboard roving focus (`ArrowLeft`, `ArrowRight`, `Home`, `End`) across `.hero-tabs` and `.sandbox-out-tabs`.
- **Interactive Sandbox (`#sandbox`)**: Pair a preset selector, reset button, and live `secret_scan` status badge with a two-pane input/output grid. Bind the global `/` key to focus `#sandbox-input` when not already typing in an editable element.
- **Motion & Feedback**: Use `--ease` (`cubic-bezier(0.16, 1, 0.3, 1)`) for UI transitions, a `2px` top `.scroll-progress` bar driven by `animation-timeline: scroll()`, and pure-CSS shimmer/border-beam effects wrapped in `@media (prefers-reduced-motion: no-preference)`. Disable all transitions and animations under `@media (prefers-reduced-motion: reduce)`.
