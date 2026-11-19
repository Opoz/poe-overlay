# Releasing

Releases are built by GitHub Actions (`.github/workflows/release.yml`) when a
version tag is pushed. The installer is unsigned (no paid code-signing
certificate), so Windows SmartScreen warns on first install. Updates are
signed with the updater key, and the app refuses any update that isn't.

## One-time setup

The updater key pair lives outside the repo at `~/.tauri/poe-overlay.key`
(private) and `~/.tauri/poe-overlay.key.pub` (public, also in
`src-tauri/tauri.conf.json` under `plugins.updater.pubkey`). Keep a backup of
the private key: without it, installed copies can't be updated.

Add the private key to the repository's secrets:

```sh
gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/poe-overlay.key
```

The key has no password, so `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` can stay unset.

## Each release

1. Merge `dev` into `main` once the build has been checked in game.
2. Bump `version` in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` and
   `package.json`, commit to `main`.
3. Tag it and push the tag: `git tag v0.2.0 && git push origin v0.2.0`.
4. The workflow drafts a GitHub release with the installer and
   `latest.json`. Check the draft, then publish it. Running copies offer the
   update at their next start.

## Building locally

`pnpm tauri build` signs the updater files, so it needs the key:

```sh
TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/poe-overlay.key)" pnpm tauri build
```
