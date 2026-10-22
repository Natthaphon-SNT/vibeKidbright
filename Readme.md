# VibeKidbright IDE 3.7.1 ? Classic KidBright hotfix

Based on the supplied 3.7.0 source, byte-identical to tag v3.7.0. This is an isolated maintenance branch, not a downgrade of the current 3.7.4 development workspace.

Fixes managed toolchain activation/Python paths/cache refresh without restarting the app, and isolates explicit new-project requests from an already-open project. Includes current classic hardware references and a refreshed compiled system prompt. No ?AI/?AI Plus target selection, ADB commands, learning workspace or deployment is included.

See [release notes](docs/releases/3.7.1.md). Source, lockfiles and installer metadata all use 3.7.1. The Windows release workflow targets hotfix/3.7.1 and creates a draft with make_latest=false.

## Build and check

Use a Node version accepted by package.json and the Tauri build prerequisites for the target OS.

```sh
npm ci
npm test
npm run check:hotfix
npm run build:release
cd src-tauri
cargo test --lib --locked --offline
cd ..
npm run tauri -- build --bundles nsis --ci
```

The existing automatic toolchain download remains enabled. The hotfix corrects activation order and environment discovery; it does not bundle the large ESP-IDF toolchain into the installer.

Tests use local fixtures and the compiled UI with simulated IPC. No live AI inference, real toolchain installation, hardware flash, installer-over-existing-install test or macOS installer qualification was performed by this hotfix preparation. Validate the Windows upgrade from 3.7.0 on a separate machine/Sandbox before public distribution.
