# Project Mobius

Mobius is a cross-platform desktop app that watches your Downloads folder,
detects newly completed files, and asks how you'd like to organize them. Move
files to sensible category folders, set up remembered rules for the types you
care about, extract archives, and undo anything with a click.

It is local-only: no network, no telemetry, no accounts. Mobius never deletes
your files.

## Features

- **Smart monitoring** — native filesystem events with debounce, a readiness gate
  that avoids moving files still being written, and a periodic reconciliation scan.
- **Extension classification** — files map to destinations such as
  `Documents/PDFs`, `Documents/Word Files`, `Images`, and `Apps` for disk images
  and macOS bundles.
- **Simple confirmation dialog** — move to the suggested folder, choose a custom
  folder, remember a rule for that file type, or do nothing.
- **Batch handling** — files downloaded together are grouped, with an "apply to
  all current downloads" option.
- **Archive extraction** — opt-in, immediate `Extract to [Browse…]` for
  `.zip`/`.tar`/`.tar.gz`/`.tgz`.
- **Duplicate versioning** — conflicting files move to a `Previous Versions/`
  subfolder instead of being overwritten.
- **Rules manager & overrides** — inspect learned rules and tune destinations by
  extension or whole category.
- **Tray, pause/snooze, and launch at login** — quiet background operation.
- **History & undo** — reverse recent moves from a toast or the history panel.

## Tech stack

Rust + [Tauri](https://tauri.app) v2, with a React + TypeScript + Vite frontend.

## Documentation

Full specifications live in [SPECS.md](./SPECS.md).

## Status

Early development. The specification is complete; implementation is in progress.

## License

MIT — see [LICENSE](./LICENSE).
