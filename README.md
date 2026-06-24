# Copycat — AI Voice-to-MIDI

A VST3/CLAP plugin that transcribes vocal audio into MIDI notes using
[GAME](https://github.com/openvpi/GAME) (a generative AI model for multi-instrument
music transcription). Built with Gemini and Deepseek 🚀

Built with [nih-plug](https://github.com/robbert-vdh/nih-plug).

## Features

- Voice-to-MIDI transcription via ONNX neural network models
- Record audio from DAW input or load audio files (WAV, FLAC, MP3, OGG)
- Piano roll visualization with note names
- Export transcribed MIDI to file
- Adjustable BPM, segmentation, and pitch estimation parameters
- D3PM diffusion steps control for transcription quality
- Language hint support (English, Japanese, Cantonese, Mandarin)
- Works on Wine (GUI via OpenGL 2.1 compatibility)

## Build

Requires Rust and the mingw-w64 toolchain for Windows targets:

```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64  # or equivalent
```

Then:

```bash
cd copycat
cargo xtask bundle copycat --release --target x86_64-pc-windows-gnu
```

Output goes to `target/bundled/`:
- `copycat.clap` — CLAP plugin
- `copycat.vst3` — VST3 plugin (CLAP recommended on Wine)

## Notes

- The `nih-plug-patched/` directory is a local fork with OpenGL 2.1 fallback
  (GUI compatibility on Wine) and `catch_unwind` wrappers on FFI entry points.
- The ONNX transcription engine (`ort`) is loaded dynamically at runtime;
  `onnxruntime.dll` must be present next to the plugin when transcribing.
