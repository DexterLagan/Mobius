# Mobius — Specifications

## 1. Overview

Mobius is a cross-platform desktop application that monitors the user's Downloads
folder, detects newly completed files, classifies them by type, and asks the user
how each file should be organized. The user can move the file to the suggested
category folder, choose a custom folder, apply a remembered rule, or leave the file
in place.

Mobius is a local-only application. It performs no network communication, contains
no telemetry, and never deletes files.

## 2. Goals and Non-Goals

### Goals
- Detect newly downloaded files reliably without moving files that are still being written.
- Classify files by extension into a small, predictable set of categories.
- Present a single, non-spammy UI for confirming organization actions.
- Allow per-extension and per-category customization, including remembered rules.
- Provide undo for recent moves.
- Behave consistently on Windows, macOS, and Linux.

### Non-Goals
- Cloud sync, accounts, or any network feature.
- Content-based analysis (MIME sniffing, malware scanning).
- Managing or organizing files outside the Downloads folder.
- File deletion or renaming in place. The only non-move operation is opt-in
  archive extraction (§5.5), and the source archive is never deleted.
- Background operation as a system service (the app runs as a tray app, §5.9).

## 3. Locked Technical Stack

| Layer            | Choice                                                        |
| ---------------- | ------------------------------------------------------------- |
| Core / backend   | Rust (edition 2021)                                           |
| App framework    | Tauri v2                                                      |
| Frontend         | React + TypeScript + Vite                                     |
| File watching    | `notify` crate (native FS events) with debounce               |
| Downloads lookup | `dirs` crate                                                  |
| Config storage   | Tauri Store plugin (`tauri-plugin-store`), JSON on disk       |
| Tray / background| Tauri tray icon API                                           |
| Launch at login  | `tauri-plugin-autostart`                                      |
| Archive extract  | `zip` + `tar` + `flate2` crates (zip/tar/gz only)             |
| Async runtime    | Tokio (Tauri's runtime)                                       |
| Persistence      | Local JSON under the OS app-config directory                  |
| Packaging        | `tauri build` (MSI/NSIS, DMG, AppImage/deb)                   |

Single binary, one shared UI codebase rendered in each platform's native webview.

## 4. Architecture

```
┌──────────────────────────── Tauri App ────────────────────────────┐
│                                                                    │
│  Rust core                                                         │
│  ├── Watcher (notify + debounce)                                   │
│  ├── Readiness gate (temp-file + size-stability checks)            │
│  ├── Classifier (extension → category)                             │
│  ├── Mover (create dir, version collisions, atomic move)           │
│  ├── Extractor (opt-in, archive → folder; zip/tar/gz only)         │
│  ├── Rule engine (remembered rules, per-extension overrides)       │
│  ├── History (undo stack, in-memory + optional JSON)               │
│  ├── Tray + scheduler (pause/snooze, launch at login)              │
│  └── Config (load/save, defaults, migration)                       │
│        │ emits events                      ▲ commands (invoke)     │
│        ▼                                   │                        │
│  Frontend (React/TS)                                               │
│  ├── Queue store (pending files)                                   │
│  ├── Dialog / queue panel                                          │
│  ├── Settings panel                                                │
│  └── History panel                                                 │
└────────────────────────────────────────────────────────────────────┘
```

Communication is strictly over Tauri: Rust emits events (`file:detected`,
`move:completed`, `move:failed`) and exposes `#[tauri::command]` functions the
frontend invokes (`confirm_move`, `dismiss`, `remember_rule`, `set_config`,
`undo`, `list_history`).

## 5. Functional Requirements

### 5.1 Downloads Folder Detection
- Resolve the Downloads path at startup via `dirs::download_dir()`.
- If resolution fails, show a first-run screen asking the user to pick a folder.
- The monitored folder is configurable; changes require a watcher restart.
- Log the resolved path on startup for diagnosability.

### 5.2 File Monitoring
- Primary mechanism: native filesystem events via `notify` (recursive=false, top level only).
- Events are debounced (default 750 ms) to collapse write bursts into one detection.
- A periodic reconciliation scan (default every 30 s) catches missed events and
  files that appeared while the app was not running.
- On startup, reconcile existing files but do not prompt for files older than a
  configurable age (default: only files modified after app start are promoted),
  so launch does not flood the UI.
- Ignore by default: hidden/dotfiles, files beginning with `.`, symlinks, and
  directories (including macOS `.app` bundles).
- **First run:** show a dry-run review table of files already in Downloads
  (name, size, suggested destination) with *Organize all*, *Choose ones*, or
  *Skip for now*. Nothing is moved until the user confirms.

### 5.3 Readiness Gate (in-progress downloads)
A detected path is only promoted to the queue once all hold:
1. It is a regular file at the top level of the monitored folder.
2. Its extension is not a known-incomplete extension:
   `.crdownload`, `.part`, `.partial`, `.download`, `.tmp`, `.opdownload`,
   `.fdmdownload`, `.aria2`.
3. Its size is unchanged across two consecutive checks at least 1500 ms apart.
4. It is not currently open for writing where the OS exposes that information
   (best-effort on Windows/macOS; size stability is the portable fallback).

Files that fail the gate remain watched and are re-evaluated; they are never moved.

### 5.4 Classification

Classification is by extension (case-insensitive, without the leading dot). Each
extension maps to a **default destination path relative to the monitored folder**.
Destinations may be nested; this is what the dialog's first option offers.

| Default destination (relative) | Extensions |
| ------------------------------ | ---------- |
| Documents/PDFs                 | pdf |
| Documents/Word Files           | doc, docx, odt, rtf |
| Documents/Spreadsheets         | xls, xlsx, ods, csv |
| Documents/Presentations        | ppt, pptx, odp |
| Documents/Text Files           | txt, md |
| Documents/E-books              | epub, mobi |
| Images                         | jpg, jpeg, png, gif, bmp, tiff, tif, svg, webp, heic, avif |
| Audio                          | mp3, wav, flac, aac, ogg, m4a, wma, opus |
| Video                          | mp4, avi, mkv, mov, wmv, flv, webm, m4v |
| Archives                       | zip, rar, 7z, tar, gz, bz2, xz, iso |
| Code                           | js, mjs, cjs, ts, tsx, jsx, py, java, c, h, cpp, hpp, cc, cs, go, rs, rb, php, html, css, scss, json, xml, yml, yaml, sh, sql |
| Apps                           | dmg, pkg, app |
| Installers                     | exe, msi, deb, rpm, appimage |
| Documents                      | any other document extension not listed above |
| Other                          | any extension not listed above (also files with no extension) |

- Classification is purely extension-based; no content inspection.
- **Disk images and macOS bundles default to `Apps`** (`.dmg`, `.pkg`, `.app`).
- A **"Flatten to category folders"** toggle collapses everything to top-level
  categories (`Documents/`, `Images/`, `Apps/`, …, `Other/`). Default: off, so
  the per-type destinations above apply.
- Destinations, folder names, and the extension map are user-overridable; unknown
  extensions map to `Other`.

### 5.5 User Interaction

Detected files enter a **queue**. For a single file the dialog is:

```
┌──────────────────────────────────────────────────┐
│ Organize "report.pdf"?                             │
│                                                    │
│ (•) Move to Documents/PDFs                         │
│ ( ) Move to [________________________] [Browse…]   │
│                                                    │
│ [ ] Apply to all current downloads (3)             │
│ [ ] Remember for this file type (.pdf)             │
│                                                    │
│                   [ Do Nothing ]   [   Move   ]    │
└──────────────────────────────────────────────────┘
```

For an archive the dialog adds a third action (shown when the extension is
`.zip`, `.tar`, `.tar.gz`, or `.tgz`):

```
┌──────────────────────────────────────────────────┐
│ Organize "assets.zip"?                             │
│                                                    │
│ (•) Move to Archives                               │
│ ( ) Extract to [assets/______________] [Browse…]   │
│ ( ) Move to [________________________] [Browse…]   │
│                                                    │
│ [ ] Remember for this file type (.zip)             │
│                                                    │
│              [ Do Nothing ]      [   Confirm   ]   │
└──────────────────────────────────────────────────┘
```

- **Option 1 (default, selected):** move to the suggested destination from §5.4.
- **Extract (archives only):** asks immediately and extracts into a new folder
  (`assets/` by default) inside the monitored folder; `Browse…` changes the
  target. Extraction is streamed, path-traversal entries are rejected, and the
  **source archive is kept** unless the user separately chooses to move it.
- **Option 2:** move to a custom folder. `Browse…` opens a native folder picker;
  a typed relative name resolves inside the monitored folder.
- **Apply to all current downloads (N):** applies the chosen destination and rule
  to every file currently queued, not just the shown file. Shows the live count.
- **Remember for this file type (.ext):** performs the move and persists a rule
  (§5.7) so future files of this extension move without prompting. A short inline
  preview shows the resulting rule, e.g. `Next .pdf → Documents/PDFs`.
- **Do Nothing:** dismisses the file; it stays in Downloads and is not
  re-prompted for this session.
- Keyboard: `1`/`2` select the radio, `Enter` = confirm, `Esc` = Do Nothing.
- After a successful move, show a transient **undo toast** (default 10 s).

**Batch behavior**

- Files detected within the same debounce/burst window (default 2 s) are grouped
  into one batch. Archives are treated as a single file unless the user chooses
  Extract for them.
- The batch dialog lists the files and shows the count, e.g. `4 files detected`.
- If every file in the batch shares the same suggested destination, Option 1 is
  preselected as usual. If destinations differ, a third default option appears:
  `( ) Move each file to its suggested folder`, selected by default.
- "Apply to all current downloads" is hidden in batch mode (the batch already
  defines the scope); individual files can still be expanded and edited.

A **settings window** (§5.10) edits destinations, overrides, and rules. A
**history panel** lists recent moves and offers undo.

### 5.6 File Organization (move semantics)
- Destination directories are created recursively if missing.
- **Custom folder resolution:**
  - Relative names are resolved *inside the monitored folder*.
  - Absolute paths are allowed only if the user enables "allow external
    destinations"; otherwise rejected.
  - Path traversal (`..`) is rejected; names are sanitized.
- **Duplicate handling (default = version):** if `name.ext` already exists in the
  destination, the existing file is moved to a `Previous Versions/` subfolder of
  that destination, and the new file takes the canonical `name.ext`. If a file of
  the same name already exists in `Previous Versions/`, append `_1`, `_2`, … to
  the archived copy, preserving its extension. Example:
  - Before: `Documents/PDFs/report.pdf`
  - After moving a new `report.pdf`: `Documents/PDFs/report.pdf` (new) and
    `Documents/PDFs/Previous Versions/report.pdf` (old).
  - A second conflict yields `Previous Versions/report_1.pdf`.
  - Versioned copies can carry a timestamp prefix instead (`2026-09-11_report.pdf`)
    via config.
- Alternative duplicate policies (config `duplicatePolicy`, default `version`):
  `rename` (OS-style `name (1).ext`, no version folder), `replace`, `skip`, or
  `ask` (show a per-file chooser).
- **Atomicity:** prefer `rename()`; when the destination is on a different
  filesystem, fall back to copy-then-delete, verifying size before deleting the
  source. A failed move must leave the source intact.
- After a successful move, emit `move:completed` with source, destination, and
  rule used.

### 5.7 Configuration
Persisted JSON with schema versioning:
- `watchDir`, `scanIntervalMs`, `debounceMs`, `batchWindowMs`
- `destinationMap` (ext → relative destination), `categoryNames` (category → folder name)
- `categoryDestinations` (category → relative destination; medium-granularity
  override, see below), `extensionOverrides` (ext → category)
- `flattenToCategories` (bool; default false)
- `allowExternalDestinations` (bool)
- `ignoredExtensions`, `ignoredNames`
- `rules` (ext → destination), created by "Remember"
- `duplicatePolicy` (`version` | `rename` | `replace` | `skip` | `ask`; default `version`)
- `versionTimestampPrefix` (bool; default false)
- `extractArchives` (`ask` | `never`; default `ask`)
- `launchAtLogin` (bool; default false), `startMinimizedToTray` (bool; default true)
- `undoToastMs` (default 10000), `autoDismissTimeoutMs` (0 = off)
- `pausedUntil` (timestamp or null; suspends monitoring while set)
- `schemaVersion`

**Destination precedence** (highest first): remembered rule → per-extension
override → per-category destination → built-in default. `categoryDestinations`
lets a whole category be redirected in one place (e.g. all Documents → `Docs/`)
without editing every extension; useful when flattening or for categories such as
Images/Audio/Video that have no per-type rules.

Unknown keys are preserved on save. On parse failure, back up the bad file and
load defaults.

### 5.8 History and Undo
- Keep an in-memory stack of the last 50 moves, each with source, destination,
  timestamp, and originating rule.
- `undo` moves the file back to its original path if it still exists and the
  original location is free; otherwise the undo fails with a clear message.
- History is optionally persisted to JSON so it survives restarts.
- The undo toast (§5.5) provides one-click reversal of the most recent move; the
  history panel offers the same action for earlier entries and a **Reveal in
  folder** action for the destination.

### 5.9 Tray, Pause/Snooze, and Launch at Login
- Mobius lives in the system tray / menu bar; closing the main window hides it to
  the tray rather than quitting.
- Tray menu: **Open**, **Pause** (30 min / 1 h / 3 h / until resumed), **Resume**,
  **Recent moves** (last 5, with undo), **Settings**, **Quit**.
- While paused, the watcher stays registered but detected files are not promoted
  to the queue; they are picked up on resume. `pausedUntil` stores the expiry.
- The tray icon reflects state (active / paused / error).
- **Launch at login** is toggled from settings and implemented with
  `tauri-plugin-autostart`; default off. When enabled, the app starts minimized to
  the tray (`startMinimizedToTray`, default true).
- A silent, non-blocking notification may accompany a completed auto-move when the
  window is hidden (configurable; default on for remembered rules only).

### 5.10 Settings and Rules Manager

The settings window has these sections:

- **General:** watched folder, scan interval, debounce/batch window, undo timeout,
  auto-dismiss, launch at login, start minimized.
- **Destinations:** the place where per-category and per-extension overrides are
  configured. Rendered as a category list; each category row shows one folder
  field and can be expanded to reveal its extension rows:

  ```
  ▾ Documents                    [ Documents/            ]  [Reset]
      pdf                        [ Documents/PDFs       ]
      doc, docx                  [ Documents/Word Files ]
      xls, xlsx                  [ Documents/Spreadsheets ]
      …
  ▸ Images                       [ Images/              ]
  ▸ Apps                         [ Apps/                ]
  ```

  - Editing a **category** field writes `categoryDestinations[category]` and
    changes the default for every extension in that category at once.
  - Editing an **extension** row writes the per-extension destination and
    overrides the category default for that extension only.
  - **Reset** removes the override (clearing back to the next level down).
  - A read-only indicator shows which layer currently wins for a selected
    extension (`rule → extension → category → default`).
  - The **Flatten to category folders** toggle lives here.
- **Rules:** the rules manager — a table of every learned and manually added rule
  with columns `Extension`, `Destination`, `Source` (Remember/Manual),
  `Created`, and `Enabled`. Rows support inline edit, enable/disable, delete, and
  **Add rule**. Learned rules are ordinary rows the user can inspect and change,
  so automatic moves are never a black box.
- **Exclusions:** ignored extensions and filename patterns.
- **Appearance:** theme, density, and notification preferences.

A **history panel** lists recent moves and offers undo.

### 5.11 Organize Existing Files

A button in the Downloads view opens an organizer dialog for files already in the
watched folder (not just newly detected ones). It lists every top-level file
(hidden files and ignored patterns excluded) with the destination it would move to
under the current configuration:

```
┌─────────────────────────────────────────────────────────────────┐
│ Organize existing files                                    Close │
├──┬───────────────────────┬────────────────────────┬─────────────┤
│☑ │ report.pdf            │ Documents/PDFs         │    Edit     │
│☑ │ setup.dmg             │ Apps                   │    Edit     │
│☐ │ notes.txt             │ Documents/Text Files   │    Edit     │
├──┴───────────────────────┴────────────────────────┴─────────────┤
│ 2 of 3 selected          [ Cancel ]   [ Organize selected ]     │
└─────────────────────────────────────────────────────────────────┘
```

- Columns: selection checkbox, file (name + size), destination, and an **Edit**
  action. All rows are checked by default; a header checkbox toggles all.
- **Edit** turns the destination into an editable field with an **Apply to all
  .ext files** checkbox:
  - Unchecked → the change is a one-off override for that single file only.
  - Checked → the change becomes a persisted rule (`rules[ext]`), and every other
    row with the same extension updates to match. Such rows are tagged `rule`.
- **Organize selected** moves the selected files using the shown destinations and
  the configured duplicate policy, records each move in history, and clears moved
  items from the pending queue.
- The dialog never moves anything until confirmed.

### 5.12 About

The application exposes an **About Mobius** item in the native application menu.
On macOS it uses the system About panel (name, version, author). On Windows/Linux
the menu item opens a native message dialog. It identifies the app as
"by Dexter Santucci".

## 6. Error Handling

- File busy / permission denied: leave the file in place, surface a non-blocking
  error toast, and keep the item queued.
- Destination creation failure: abort the move, report the OS error.
- Watcher failure: restart the watcher with backoff; after repeated failures,
  notify the user and fall back to reconciliation scans only.
- All errors are logged (stdout in dev, rotating file in release) and never
  crash the app or silently drop a detected file.

## 7. Platform Considerations

- **macOS:** `.app` is a directory and is ignored by default; DMG/PKG are treated
  as executable files. `dirs` respects `$HOME/Downloads`.
- **Windows:** Downloads may be redirected via the registry/shell folders; `dirs`
  handles the common case. Locked files surface as sharing violations.
- **Linux:** `XDG_DOWNLOAD_DIR` is honored; `.crdownload`-style browser temp files
  are ignored per §5.3.
- Path handling always uses normalized OS-native paths; no string concatenation
  of separators.

## 8. Security

- No network access; no telemetry.
- Tauri capabilities are minimal: only the commands and plugins actually used.
- Strict CSP for the webview; local assets only.
- Custom-folder input is validated against traversal and, by default, confined to
  the monitored folder.
- The app never executes or opens downloaded files.

## 9. Non-Functional Requirements

- **Idle footprint:** minimal CPU when no files are moving; event-driven, not
  busy-polling.
- **Latency:** a completed file appears in the queue within ~2 s of its final write.
- **Robustness:** survives restart with pending queue reconstructed from disk state.
- **Accessibility:** queue and dialog are keyboard-navigable with visible focus.
- **Internationalization-ready:** user-facing strings isolated for future translation.

## 10. Milestones

1. **M1 — Core detection:** Rust watcher + readiness gate + classifier, log-only.
2. **M2 — Move + UI:** queue dialog with suggested/custom/do-nothing, duplicate
   versioning, undo toast.
3. **M3 — Config + rules:** settings window with Destinations and Rules manager
   (§5.10), remembered rules with inline preview, per-extension and per-category
   overrides.
4. **M4 — Background + polish:** tray pause/snooze, launch at login, archive
   extraction, history/error surfacing, packaging for all three OSes.

## 11. Open Questions

- Should the periodic reconciliation scan also detect files deleted/moved
  externally, or only additions? (Current scope: additions only.)
- Should "Remember" rules support destination folders outside the monitored
  folder by default, or only when external destinations are enabled?

## 12. Suggested Enhancements (not yet committed)

Ideas worth considering after the core milestones:

- **Size-aware actions.** Skip or flag files above a configurable size, and show
  size in the dialog.
- **Download-source awareness.** Read the browser's `.download` metadata where
  available to group multi-file downloads accurately.
- **Search/filter and sort in the queue** (by type, size, time) for large batches.
- **Broader extraction formats** (rar, 7z) behind the same opt-in prompt.
- **Undo whole batch.** Reverse every move in a batch with one action.
- **Audit log export** (CSV/JSON) of all moves for transparency.