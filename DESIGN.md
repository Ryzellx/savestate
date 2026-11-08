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

No purple-blue gradients anywhere (R-01). No glow, no glassmorphism, no
background grid. Flat surfaces, one accent, generous spacing.

## Typography (R-06)

- egui default proportional font for body and headings. Reason: a utility
  is read in short bursts; legibility at small sizes matters more than a
  display face.
- Monospace only for hashes, snapshot IDs, and paths, never for headings.

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

- Page switch fades in over ~120ms. Purpose: orientation after
  navigation, nothing more. No endless pulses or loops. Reduced-motion
  setting disables the fade. No repaint when idle.

## Copy (R-02, R-16)

- Plain, specific language. No em dashes in UI text. No buzzwords.
- Safety wording is literal: restore never overwrites silently, a safety
  backup is taken first, conflicts are listed with sizes. Known
  limitations (Xbox/MS Store, EA App, Ubisoft) are stated in the app and
  the README, not hidden.

## Icons

- Text labels only in this release; no icon set is imported. Reason: the
  sidebar has eight short labels that read faster as words, and a generic
  icon font would add a recognizable library look without adding meaning
  (R-04).
