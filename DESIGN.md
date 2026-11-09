# SaveState DESIGN.md

Design direction, written from the PRD's own visual direction. Antislop
filter applied on top.

Reading this as: a desktop utility for gamers who care about their saves,
in a dark premium gaming-utility style, dials ENERGY 2 / RHYTHM 2 / MOTION 2.

## Palette (R-29: 2 core colors + 1 accent)

- Core 1, near-black `#0C0E12`: window background. Reason: long sessions
  in front of a game library read better dark, and the PRD asks for a
  modern dark interface.
- Core 2, charcoal `#14171D` / `#1B1F27`: panels and cards. Reason: one
  step lighter than the background so game rows and snapshot cards read
  as surfaces.
- Accent, warm amber `#E8B43C`: the one focal action per screen (Back up,
  Run restore), verified badges, the active wizard step. Reason: amber is
  the save-point color in games, it separates SaveState from the
  teal/green default of productivity tools, and it is used only at the
  key moment.
- Neutrals (not counted): text `#E9EBEE`, dim text `#9AA3AD`, borders
  `#2A3038`. Danger red `#E05A4E` is reserved for destructive actions
  and failed verification only.

A subtle dark vertical gradient runs behind the sidebar and the main
area (near-black top fading slightly darker toward the bottom, with a
faint amber tint along the very top edge). It is the only gradient in
the app: no purple-blue gradients anywhere (R-01), no glow, no
glassmorphism, no background grid. Cards are rounded (12px) with a
1px border and a soft shadow, so surfaces read as lifted, not flat.

## Typography (R-06)

- Headings: Space Grotesk Bold, embedded from `assets/fonts/`. Reason:
  the app's display face; it reads as a designed product instead of a
  default egui demo, while staying neutral and legible.
- Body: Inter Regular (15px), with Inter Medium available for emphasis.
  Reason: a utility is read in short bursts; legibility at small sizes
  matters more than a display face.
- Monospace: JetBrains Mono (13px) for hashes, snapshot IDs, and paths,
  never for headings.

## Layout (R-05, C-3)

- Left sidebar (collapsible) plus one content column. Reason: the user's
  job per page is one decision: dashboard = "is everything backed up",
  library = "which game", restore wizard = "what goes back, and what
  happens to the current files". The layout follows that job.
- No stat-card row with invented numbers (R-17). Dashboard shows only
  counts computed from the local database: games, snapshots, bytes stored,
  bytes saved by dedup and compression, last backup time.
- The Restore Wizard is three honest steps (choose snapshot, check
  target, preview and confirm), not a templated three-step "how it works"
  section: each step does real work (conflict listing, process check,
  disk-space check, dry run).
- Empty states name the cause and the next action (R-27): "No games
  detected yet. Press Scan for games in the sidebar."

## Motion (R-19)

- Page switch fades in over ~180ms; cards on the new page stagger in
  behind it. Sidebar collapse animates its width over ~220ms instead of
  snapping. Toasts slide in from the bottom-right and fade out after 4s.
  Purpose: orientation after navigation and confirmation of actions,
  nothing more. No endless pulses or loops. Reduced-motion setting makes
  every transition instant. No repaint when idle: the app requests a
  repaint only while an animation is in progress.

## Copy (R-02, R-16)

- Plain, specific language. No em dashes in UI text. No buzzwords.
- Safety wording is literal: restore never overwrites silently, a safety
  backup is taken first, conflicts are listed with sizes. Known
  limitations (Xbox/MS Store, EA App, Ubisoft) are stated in the app and
  the README, not hidden.

## Icons

- Each sidebar item has one small text glyph next to its label
  (Dashboard ▦, Game Library ▤, Game Details ℹ, Snapshots ◈, Restore ↺,
  Storage ▣, Automation ◷, Settings ⚙). Reason: the glyphs mark each
  destination at a glance; they render from the bundled egui fallback
  fonts, so no icon set is imported (R-04). No emoji.
