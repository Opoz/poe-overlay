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
