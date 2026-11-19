# poe-overlay

Path of Exile 1 levelling companion: Tauri v2 + SolidJS. See `docs/PLAN.md` for scope and milestones.

- Use the vocabulary in `GLOSSARY.md` in code, comments and UI text. Update it when a term is resolved.
- Decisions live in `docs/adr/`. Read them before changing architecture.
- **Independent implementation:** Exile UI's source may be read to understand behaviour, but never copy code from it. Only exile-leveling data is vendored (`THIRD_PARTY.md`).
- **Game safety:** never read game memory, inject, or send input to the game. Clipboard reads happen only on a manual hotkey press.
- Domain logic goes in `crates/poe-core` (no Tauri deps); `src-tauri` is wiring; the frontend only renders.
- `.local/` holds private test data (real Client Logs with chat). Never commit it. Test fixtures must be trimmed and scrubbed of chat/whispers.

## Commands

- `pnpm tauri dev`: run the app
- `cargo test --workspace`: Rust tests
- `pnpm test`: frontend tests
- `pnpm check`: Biome lint/format + `tsc`
- `cargo clippy --workspace -- -D warnings` / `cargo fmt --all`
- `pnpm gate` (`--fix` to format first, `--quick` for format/lint/types only): every CI check in order. Git hooks in `.githooks/` (enabled by `pnpm install`) run `--quick` on commit and the full gate on push; don't bypass them with `--no-verify`
- `pnpm preview:shots [dir] [filter]`: regenerate `src/preview/fixtures.json` from the core, then screenshot every window via `preview.html` (needs `pnpm dev`; `--no-fixtures` to skip)
- `pnpm build:signed`: local release build signed with the updater key (`docs/RELEASING.md`)
- `pnpm icon`: re-render `assets/icon.svg` and regenerate `src-tauri/icons/`
- New recurring tasks get a script in `scripts/` and a line here, not a one-off shell command

## Branches

- `docs/IN-GAME-CHECKS.md` queues what still needs checking in the real game; add an entry for every user-facing change, and tick items off only when the player confirms.

- Work happens on `dev` (CI runs there). `main` only gets changes after they have been verified in-game; merge `dev` into `main` once the player confirms.
