# NFO Memo

A native, cross-platform NFO editor and reader built with Rust and Slint. No Electron, no browser runtime, no telemetry.

## Features

- Byte-accurate IBM code page 437 import/export, including box drawing, blocks, shades, and symbols
- UTF-8 and Windows-1252 compatibility modes
- CRLF, LF, and classic CR line-ending control
- Editor and distraction-free reader views with an embedded **PxPlus IBM VGA8** font
- Native dark-mode controls, real application menus, and keyboard shortcuts
- Font selector for PxPlus IBM VGA8 and installed system monospace fonts
- Scene release, technical readme, `file_id.diz`, and BBS bulletin templates
- One-click box, rule, shade, and tag snippets
- Visual NFO Builder with a real editable box, live alignment and border styling, fixed canvas/box widths, an exact CP437 preview, and one-click conversion into the NFO document
- Native PNG and WebP rendering using the selected font, with PxPlus IBM VGA8 as the fallback
- ASCII-armored detached GPG signatures, secret-key selection, signature verification, and a persistent default key
- Native file dialogs and a native Slint UI on Windows, macOS, and Linux
- 80-column ruler plus live line, character, and widest-line counters

## Run it

```sh
cargo run
```

GPG signing uses the `gpg` executable on `PATH` and your normal GnuPG keyring. Exporting and editing work without GPG.

## Test and build

```sh
cargo test
cargo build --release
```

The release binary is written to `target/release/nfo-memo` (or `nfo-memo.exe` on Windows).

For OS-native bundles, install [`cargo-bundle`](https://github.com/burtonageo/cargo-bundle) and build on each target operating system:

```sh
cargo install cargo-bundle
cargo bundle --release
```

## Encoding behavior

NFO Memo defaults to CP437. Change the encoding before opening a file to control decoding, or before saving to control output. Unsupported CP437 characters are replaced with `?` and reported in the status bar. GPG signs the exact byte representation produced by the selected encoding and line-ending mode.

## Font

The bundled `PxPlus_IBM_VGA8.ttf` comes from VileR's **The Ultimate Oldschool PC Font Pack** and is licensed under CC BY-SA 4.0. See [`assets/FONT-LICENSE.txt`](assets/FONT-LICENSE.txt).

## License

NFO Memo is licensed under **GPL-3.0-only**. The bundled font is distributed separately under CC BY-SA 4.0 as described above. Third-party Rust dependencies retain their own licenses.
