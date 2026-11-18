# poe-overlay

A lightweight Path of Exile 1 levelling companion for Windows: campaign guide, act splits timer, Path of Building import with gem and passive-tree overlays, vendor regex, bandit choice and lab layouts, drawn as click-through overlays over the game.

Built with Rust + [Tauri v2](https://tauri.app) and SolidJS. Not affiliated with Grinding Gear Games.

> **Status:** early development. See [docs/PLAN.md](docs/PLAN.md).

## Requirements

- Windows 10/11
- Path of Exile in **windowed fullscreen / borderless** mode

## How it works

It reads only the game's `Client.txt` log. No memory reading, no injection, no input sent to the game.

## Development

```sh
pnpm install
pnpm tauri dev
cargo test --workspace
```

## Credits

See [THIRD_PARTY.md](THIRD_PARTY.md). Licensed under [MIT](LICENSE).
