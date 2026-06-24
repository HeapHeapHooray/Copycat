# Copycat — AI Voice-to-MIDI

A VST3/CLAP plugin that transcribes vocal audio into MIDI notes using
[GAME](https://github.com/openvpi/GAME).

Built with [nih-plug](https://github.com/robbert-vdh/nih-plug).

## Features

- Voice-to-MIDI transcription via ONNX neural network models
- Record audio from DAW input or load audio files (WAV, FLAC, MP3, OGG)
- Real-time MIDI note output during DAW playback
- Piano roll visualization with note names
- Export MIDI to file or temp folder for drag-and-drop into DAW
- OLE drag-and-drop on Windows
- Adjustable BPM, segmentation, pitch estimation, and diffusion steps
- Language hint support (English, Japanese, Cantonese, Mandarin)
- Works on Wine (GUI via OpenGL 2.1 compatibility)

## Pre-built binaries

Download from [GitHub Releases](https://github.com/openvpi/Copycat/releases).

## Build

### Linux

```bash
cargo xtask bundle copycat --release
```

### Windows (native, recommended)

```powershell
cargo xtask bundle copycat --release --target x86_64-pc-windows-msvc
```

### Windows (cross-compile from Linux)

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cargo xtask bundle copycat --release --target x86_64-pc-windows-gnu
```

`onnxruntime.dll` is downloaded automatically by the build script for Windows targets.

### GitHub Actions

Push to `main` or open a PR — CI builds both Linux and Windows
(`.github/workflows/build.yml`).

## Output

`target/bundled/` contains:
- `copycat.clap` — CLAP plugin (recommended on Wine)
- `copycat.vst3` — VST3 plugin
- `onnxruntime.dll` — ONNX Runtime (Windows only)

## Notes

- `nih-plug-patched/` is a local fork with OpenGL 2.1 fallback for Wine
  and `catch_unwind` wrappers on all FFI entry points.
- ONNX Runtime is loaded dynamically; place `onnxruntime.dll` next to the
  plugin when transcribing (auto-downloaded during build).
