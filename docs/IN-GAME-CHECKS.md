# In-game checks

What's built but not yet verified in the real game. Tick items off as they're
checked; anything that fails becomes a bug to fix on `dev`. `main` only gets
`dev` once these pass (see CLAUDE.md, Branches).

Run the game in windowed fullscreen. Start the app with `pnpm tauri dev`.

## Setup

- [ ] **Updater secret**: `gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/poe-overlay.key`, and back up the key file (docs/RELEASING.md).
- [ ] **Settings carry over**: old settings load. Panel Opacity starts at 85% (the old opacity setting was renamed).

## Main window

- [ ] Sidebar pages all open. The run strip's clock ticks and pauses at character select.
- [ ] **Characters**: the guide options (league start, bandits) apply to the character being played.
- [ ] **History**: bars look right for real runs. Clicking a run shows its splits; deleting one updates the best run and comparisons.

## Overlays (all)

- [ ] **Panel opacity**: at 20%, text stays sharp and readable over bright areas (e.g. Act 1 beach, Act 6 shore).
- [ ] **Size**: 70% and 150% look right on each overlay. Auto-sized ones (guide, gems, XP, notes, layouts, lab, tree bar) re-fit; none are cut off or have extra blank space.
- [ ] **Showing**: unticking the timer, guide, gem links, XP ruler, notes or layouts hides them, and they stay that way after a restart.
- [ ] **Reset positions** moves every overlay back.

## Shortcuts

- [ ] Rebind a few. The old keys stop working, the new ones work, and keycaps and tooltips everywhere show the new keys.
- [ ] A combination already in use is refused with the other action's name. Plain letters are refused; F-keys are allowed.
- [ ] Clear, Default and Reset all shortcuts all work. The passive tree key field still works.

## Build notes (Alt+N)

- [ ] Re-import a PoB build with notes; its colours show.
- [ ] The mouse wheel scrolls the notes without zooming the game. It never takes focus from the game.

## Zone layouts (Alt+Z)

- [ ] Put a few images in the layouts folder ("Open layouts folder" on the Overlays page) named by area ID. They show on entering that area and not elsewhere.
- [ ] Clicking one keeps it large and fades the rest; the pick resets in the next area.
- [ ] Images in a folder named after an area ID also work.

## Act 1 layout schematics (pilot)

- [ ] `pnpm layouts`, then play Act 1. On entering each zone you see its variants; clicking one shows its follow-up steps; "All layouts" goes back.
- [ ] Compare our drawings with the references side by side (comparison page from the session) and note any that are wrong or unhelpful.
- [ ] Decide: do the other acts this way, or not.

## Screen copies

- [ ] Add one, switch to the game, drag a box over the flasks. The copy appears and updates live. If it's blank, the click-through overlay window style is the first suspect.
- [ ] Move it in Interact Mode; it stays there after locking and after a restart.
- [ ] Size, opacity, rename, "Pick area" again, and delete all work. Right-click cancels picking.
- [ ] No noticeable FPS cost with a few copies running.

## Atlas overlay (Alt+A, calibrate Alt+Shift+A)

- [ ] Import an official pathofexile.com atlas tree link.
- [ ] Two-click calibration: the start in the centre, then the named keystone at the top. Rings sit around the planned nodes. If they're off, check whether the centre hub is really where the game draws the start.
- [ ] After that, one click (or a middle-click on the start) lines it up again after panning.
- [ ] Rings never cover the nodes, and lit / unlit is easy to see through them.

## Lab

- [ ] The map has no empty space underneath and the window fits it.
- [ ] On a day with a golden door, the gold key and padlock show in gold. Check that poelab's file says "golden-door" or similar; the app assumes the word "gold" is in it.

## Release (when ready)

- [ ] `pnpm build:signed` builds an installer that installs and runs.
- [ ] Pushing a version tag drafts a release with the installer and `latest.json`. After publishing, an older install offers the update.
