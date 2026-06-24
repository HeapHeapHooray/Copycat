# Copycat — AI Voice-to-MIDI

A VST3/CLAP plugin that transcribes vocal audio into MIDI notes using
[GAME (Generative Adaptive MIDI Extractor)](https://github.com/openvpi/GAME).

![Copycat GUI](assets/copycat_GUI.png)

Built with [nih-plug](https://github.com/robbert-vdh/nih-plug).

## Features

- **AI Transcription**: Voice-to-MIDI transcription via ONNX neural network models.
- **Audio Inputs**: Record audio from DAW input or load audio files (WAV, FLAC, MP3, OGG).
- **Real-time Playback**: Real-time MIDI note output during DAW playback.
- **Visual Piano Roll**: Visual interface with note names.
- **📋 Copy to Clipboard**: Copy transcribed MIDI notes directly to the clipboard (uses standard MIDI file bytes, fully compatible with FL Studio's "Paste from MIDI clipboard" feature).
- **Flexible Parameters**: Adjustable BPM, segmentation, pitch estimation, and diffusion steps.
- **Language Hints**: Support for English, Japanese, Cantonese, and Mandarin.
- **Cross-Platform & Wine Compatibility**: Works on Windows and Linux, including Wine compatibility (GUI rendering fallback to OpenGL 2.1).

## Installation

Download the release archive for your platform from [GitHub Releases](https://github.com/openvpi/Copycat/releases).

The package includes a one-click CLI installer (`copycat_installer`) that automates setting up the plugin and downloading the required model files.

### Windows
1. Extract the release ZIP.
2. Run `copycat_installer.exe`.
   - *Note: If you run it normally, it will install the plugin to your user-local directories. Run it as Administrator if you want to install it system-wide.*
3. The installer will:
   - Copy `copycat.clap` to your CLAP directory (e.g. `C:\Program Files\Common Files\CLAP`).
   - Copy `copycat.vst3` directory to your VST3 directory (e.g. `C:\Program Files\Common Files\VST3`).
   - Download the model checkpoint (`GAME-1.0.3-large-onnx`) and extract it to:  
     `C:\Users\<YourUsername>\copycat\models\GAME-1.0.3-large-onnx`

### Linux
1. Extract the release tarball.
2. Open a terminal, navigate to the extracted directory, and run the installer:
   ```bash
   ./copycat_installer
   ```
3. The installer will:
   - Copy `copycat.clap` to `~/.clap/`
   - Copy `copycat.vst3` to `~/.vst3/`
   - Download the model checkpoint (`GAME-1.0.3-large-onnx`) and extract it to:  
     `~/.local/share/copycat/models/GAME-1.0.3-large-onnx`

---

## Building from Source

### Prerequisites
Make sure you have Rust and Cargo installed.

### 1. Compile the Plugin

#### Linux
```bash
cargo xtask bundle copycat --release
```

#### Windows (native, recommended)
```powershell
cargo xtask bundle copycat --release --target x86_64-pc-windows-msvc
```

#### Windows (cross-compile from Linux)
```bash
rustup target add x86_64-pc-windows-gnu
sudo apt install mingw-w64
cargo xtask bundle copycat --release --target x86_64-pc-windows-gnu
```

### 2. Compile the Installer
Build the installer package for your target architecture:

#### Linux
```bash
cargo build --package copycat_installer --release
cp target/release/copycat_installer target/bundled/
```

#### Windows (native)
```powershell
cargo build --package copycat_installer --release --target x86_64-pc-windows-msvc
Copy-Item target/x86_64-pc-windows-msvc/release/copycat_installer.exe target/bundled/
```

`target/bundled/` will now contain the complete package:
- `copycat_installer` (or `copycat_installer.exe`)
- `copycat.clap`
- `copycat.vst3/`

### GitHub Actions Build
The project uses GitHub Actions (`.github/workflows/build.yml`) to automatically compile the plugin and installer binaries for both platforms on every tag release (e.g. pushing a tag starting with `v*`).

---

## Notes

- `nih-plug-patched/` is a local fork with OpenGL 2.1 fallback for Wine compatibility and `catch_unwind` wrappers on all FFI entry points to prevent host crashes.
