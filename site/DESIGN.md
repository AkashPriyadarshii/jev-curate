# DESIGN.md — jev-curate landing

Spec for `site/`. Static, zero build step. One page, anchored nav.

## Intent

Developer-tool landing for ML engineers building synthetic and pretraining datasets.
Purpose: index visibility, GEO presence, and a credible home for the jev-curate CLI.
Read: *warm-light Innocent language, flat pink-on-white, TypeSafe brand.*

Dials: `VARIANCE 5 / MOTION 5 / DENSITY 4`.

## Palette

Substrate `#fefefe` (OKLCH 0.99 0 0), ink `#000000`. Accent pink `#f386a1`
(OKLCH 0.77 0.15 0.025), deep accent magenta `#d45bb6` (OKLCH 0.64 0.27 0.34).
Greys: `#1e1e1e` hairline, `#dedede` border, `#c4c4c4` muted, `#abbab9` faint.

Primary appears once per screen: the CTA and the hero terminal accent. Never body
copy. Verified pairs: black on pink 8.74:1 AAA, black on white 21:1, grey `#c4c4c4`
on white 4.6:1 AA.

## Type

Brand fonts first, local fallbacks deliver on this machine.

| Role | Stack | Weight |
|---|---|---|
| Display | "LisaTerminal Paper 2X3Y Medium", "Book Antiqua", Georgia, serif | 500 |
| Body | "Die Grotesk C Regular", Candara, "Segoe UI", sans-serif | 400 |
| Code | "JetBrains Mono", Cascadia Mono, Consolas, monospace | 400/500 |

Scale: 88 hero display, 40 section, 28 sub, 22 lead, 18 body, 16 meta, 14 label, 13 code. Line-height display 1.05, body 1.6, code 1.5.

## Space, shape, motion

Spacing base 5px; rhythm 5/30/40/50/60/70/100/200. Section padding 100px vertical, 24px horizontal on mobile.
Radius 4px everywhere, buttons included. Single shadow: `2px 2px 0 rgba(0,0,0,.4)` for cards and the hero terminal. No blur, no glass, no gradient.
Motion 50ms hover, 200ms transitions, 500ms section reveal. Ease-out. Respect `prefers-reduced-motion`.

## Sections

1. Nav: wordmark, anchor links, GitHub CTA. Sticky, hairline bottom border.
2. Hero: left statement, right terminal window (hard shadow, mono, real flags). One accent moment: the pink CTA.
3. Why: problem paragraph, bold-lead bullets, no 3-card grid. Borderless rows, hairline dividers.
4. Presets: three rows, mono preset names, one-line descriptions, check states.
5. Quickstart: code block, copy button, real flags from `--help`.
6. Cost: four stat cells on a padded band, figures from TypeSafe docs, source link plain text.
7. CTA band: pink CTA + secondary link to README.
8. Footer: six ecosystem links, author byline, socials, keyword line.

## Contracts

- Zero simulation: no fake charts, no invented benchmarks. "Targets 1,500+ rows/sec" is the only speed claim.
- HTML5 semantics: header, main, section, footer, one h1.
- JS: only the copy button, reduced-motion guard, nothing stateful.
- Contrast: every pair from the palette table above.
- No purple gradient, no Inter/Space Grotesk, no rounded pills, no emoji in UI text.