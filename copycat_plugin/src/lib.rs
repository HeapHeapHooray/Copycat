use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState};
use nih_plug_egui::egui;
use ort::session::{Session, SessionInputValue};
use ndarray::{Array0, Array1, Array2};
use std::borrow::Cow;
use ort::value::Tensor;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use parking_lot::Mutex;

// Note struct representing transcribed voiced notes
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteInfo {
    pub onset: f32,
    pub offset: f32,
    pub pitch: f32,
}

// Persistable setting for non-automated parameter
#[derive(Serialize, Deserialize, Clone)]
pub struct CopycatSettings {
    pub model_dir: String,
}

#[derive(Params)]
pub struct CopycatParams {
    #[id = "tempo"]
    pub tempo: FloatParam,

    #[id = "seg_threshold"]
    pub seg_threshold: FloatParam,

    #[id = "est_threshold"]
    pub est_threshold: FloatParam,

    #[id = "nsteps"]
    pub nsteps: IntParam,

    #[id = "t0"]
    pub t0: FloatParam,

    #[id = "language"]
    pub language: IntParam,

    // Store the model directory (persisted in project session)
    #[persist = "model_dir"]
    pub model_dir: parking_lot::Mutex<String>,

    // Editor state managing window size and scaling
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
}

impl Default for CopycatParams {
    fn default() -> Self {
        Self {
            tempo: FloatParam::new(
                "Tempo",
                120.0,
                FloatRange::Linear {
                    min: 20.0,
                    max: 360.0,
                },
            )
            .with_unit(" BPM"),

            seg_threshold: FloatParam::new(
                "Segmentation Threshold",
                0.2,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            ),

            est_threshold: FloatParam::new(
                "Estimation Threshold",
                0.2,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            ),

            nsteps: IntParam::new(
                "D3PM Steps",
                8,
                IntRange::Linear {
                    min: 1,
                    max: 32,
                },
            ),

            t0: FloatParam::new(
                "Starting T",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1.0,
                },
            ),

            language: IntParam::new(
                "Language Hint",
                0,
                IntRange::Linear {
                    min: 0,
                    max: 4,
                },
            ),

            model_dir: parking_lot::Mutex::new(String::new()),

            editor_state: EguiState::from_size(750, 600),
        }
    }
}

// Shared state between GUI and Audio Thread
struct CopycatSharedState {
    recording_buffer: Mutex<Vec<f32>>,
    transcribed_notes: Mutex<Option<Vec<NoteInfo>>>,
    status: Mutex<String>,
    is_transcribing: AtomicBool,
    recording: AtomicBool,
    sample_rate: AtomicU32,
}

pub struct Copycat {
    params: Arc<CopycatParams>,
    shared_state: Arc<CopycatSharedState>,
}

impl Default for Copycat {
    fn default() -> Self {
        Self {
            params: Arc::new(CopycatParams::default()),
            shared_state: Arc::new(CopycatSharedState {
                recording_buffer: Mutex::new(Vec::new()),
                transcribed_notes: Mutex::new(None),
                status: Mutex::new("Idle. Record from DAW or load audio file to begin.".to_string()),
                is_transcribing: AtomicBool::new(false),
                recording: AtomicBool::new(false),
                sample_rate: AtomicU32::new(44100),
            }),
        }
    }
}

impl Plugin for Copycat {
    const NAME: &'static str = "Copycat";
    const VENDOR: &'static str = "Antigravity";
    const URL: &'static str = "https://github.com/openvpi/GAME";
    const EMAIL: &'static str = "info@openvpi.org";
    const VERSION: &'static str = "0.1.0";

    type SysExMessage = ();
    type BackgroundTask = ();

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames {
                main_input: Some("Input"),
                main_output: Some("Output"),
                aux_inputs: &[],
                aux_outputs: &[],
                layout: None,
            },
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames {
                main_input: Some("Input"),
                main_output: Some("Output"),
                aux_inputs: &[],
                aux_outputs: &[],
                layout: None,
            },
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn reset(&mut self) {}

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let shared_state = self.shared_state.clone();

        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::default().fill(egui::Color32::from_rgb(18, 18, 22)).inner_margin(15.0))
                    .show(egui_ctx, |ui| {
                        // Title / Header Section
                        ui.vertical_centered(|ui| {
                            ui.heading(
                                egui::RichText::new("C O P Y C A T")
                                    .font(egui::FontId::proportional(26.0))
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 230, 180)),
                            );
                            ui.label(
                                egui::RichText::new("AI Voice-to-MIDI Transcriber (based on GAME.rs)")
                                    .font(egui::FontId::proportional(11.0))
                                    .color(egui::Color32::from_gray(140)),
                            );
                        });

                        ui.add_space(10.0);

                        // Main Sections split in two columns
                        ui.columns(2, |cols| {
                            // Column 1: Config & Model Settings
                            cols[0].vertical(|ui| {
                                ui.group(|ui| {
                                    ui.set_min_height(200.0);
                                    ui.heading(egui::RichText::new("⚙ Settings").font(egui::FontId::proportional(14.0)).strong());
                                    ui.add_space(5.0);

                                    // Model Dir Picker
                                    ui.horizontal(|ui| {
                                        ui.label("Model Path:");
                                        if ui.button("Browse...").clicked() {
                                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                                *params.model_dir.lock() = path.to_string_lossy().to_string();
                                            }
                                        }
                                    });
                                    let current_model_dir = params.model_dir.lock().clone();
                                    let display_path = if current_model_dir.is_empty() {
                                        "Please select the model folder".to_string()
                                    } else if current_model_dir.len() > 35 {
                                        format!("...{}", &current_model_dir[current_model_dir.len() - 32..])
                                    } else {
                                        current_model_dir.clone()
                                    };
                                    ui.label(
                                        egui::RichText::new(&display_path)
                                            .font(egui::FontId::monospace(10.0))
                                            .color(egui::Color32::from_gray(120)),
                                    );

                                    ui.separator();

                                    // Param Sliders
                                    let mut val_tempo = params.tempo.value();
                                    if ui.add(egui::Slider::new(&mut val_tempo, 20.0..=360.0).text("BPM")).changed() {
                                        setter.set_parameter(&params.tempo, val_tempo);
                                    }

                                    let mut val_seg = params.seg_threshold.value();
                                    if ui.add(egui::Slider::new(&mut val_seg, 0.0..=1.0).text("Seg Thresh")).changed() {
                                        setter.set_parameter(&params.seg_threshold, val_seg);
                                    }

                                    let mut val_est = params.est_threshold.value();
                                    if ui.add(egui::Slider::new(&mut val_est, 0.0..=1.0).text("Pitch Thresh")).changed() {
                                        setter.set_parameter(&params.est_threshold, val_est);
                                    }

                                    let mut val_nsteps = params.nsteps.value();
                                    if ui.add(egui::Slider::new(&mut val_nsteps, 1..=32).text("D3PM Steps")).changed() {
                                        setter.set_parameter(&params.nsteps, val_nsteps);
                                    }

                                    let mut val_t0 = params.t0.value();
                                    if ui.add(egui::Slider::new(&mut val_t0, 0.0..=1.0).text("Start T")).changed() {
                                        setter.set_parameter(&params.t0, val_t0);
                                    }

                                    let mut val_lang = params.language.value();
                                    let lang_names = ["Auto/None", "English", "Japanese", "Cantonese", "Mandarin"];
                                    egui::ComboBox::from_label("Language")
                                        .selected_text(lang_names[val_lang as usize])
                                        .show_ui(ui, |ui| {
                                            for i in 0..5 {
                                                if ui.selectable_value(&mut val_lang, i, lang_names[i as usize]).changed() {
                                                    setter.set_parameter(&params.language, val_lang);
                                                }
                                            }
                                        });
                                });
                            });

                            // Column 2: Recording and Audio Control
                            cols[1].vertical(|ui| {
                                ui.group(|ui| {
                                    ui.set_min_height(200.0);
                                    ui.heading(egui::RichText::new("🎙 Audio Source").font(egui::FontId::proportional(14.0)).strong());
                                    ui.add_space(5.0);

                                    ui.horizontal(|ui| {
                                        let is_rec = shared_state.recording.load(Ordering::Relaxed);
                                        let btn_text = if is_rec { "⏹ Stop Recording" } else { "🔴 Record from DAW" };
                                        let btn_color = if is_rec {
                                            egui::Color32::from_rgb(255, 60, 60)
                                        } else {
                                            egui::Color32::from_rgb(180, 40, 40)
                                        };

                                        if ui.add(egui::Button::new(
                                            egui::RichText::new(btn_text).color(egui::Color32::WHITE)
                                        ).fill(btn_color)).clicked() {
                                            if is_rec {
                                                shared_state.recording.store(false, Ordering::Relaxed);
                                                let count = shared_state.recording_buffer.lock().len();
                                                *shared_state.status.lock() = format!("Stopped recording. Captured {} samples.", count);
                                            } else {
                                                shared_state.recording_buffer.lock().clear();
                                                shared_state.recording.store(true, Ordering::Relaxed);
                                                *shared_state.status.lock() = "Recording incoming audio... Play the track in DAW!".to_string();
                                            }
                                        }

                                        if ui.button("📂 Load Audio File...").clicked() {
                                            let state_clone = shared_state.clone();
                                            std::thread::spawn(move || {
                                                let path = match rfd::FileDialog::new()
                                                    .add_filter("Audio Files", &["wav", "flac", "mp3", "ogg"])
                                                    .pick_file()
                                                {
                                                    Some(p) => p,
                                                    None => return,
                                                };
                                                *state_clone.status.lock() = "Decoding audio file...".to_string();
                                                match load_audio(&path) {
                                                    Ok((samples, sr)) => {
                                                        *state_clone.recording_buffer.lock() = samples;
                                                        state_clone.sample_rate.store(sr, Ordering::SeqCst);
                                                        *state_clone.status.lock() = format!("Loaded audio file: {} ({} Hz)", path.file_name().unwrap().to_string_lossy(), sr);
                                                    }
                                                    Err(e) => {
                                                        *state_clone.status.lock() = format!("Error decoding: {}", e);
                                                    }
                                                }
                                            });
                                        }
                                    });

                                    ui.add_space(5.0);
                                    if ui.button("🗑 Clear Audio").clicked() {
                                        shared_state.recording_buffer.lock().clear();
                                        *shared_state.status.lock() = "Audio cleared.".to_string();
                                    }

                                    ui.add_space(5.0);
                                    // Render dynamic waveform
                                    let samples = shared_state.recording_buffer.lock().clone();
                                    draw_waveform(ui, &samples, 300.0, 95.0);
                                });
                            });
                        });

                        ui.add_space(10.0);

                        // Transcription Button & Status Group
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                let is_transcribing = shared_state.is_transcribing.load(Ordering::Relaxed);
                                let transcribe_btn = if is_transcribing {
                                    ui.add_enabled(false, egui::Button::new("⚡ Transcribing..."))
                                } else {
                                    ui.add(egui::Button::new(
                                        egui::RichText::new("⚡ TRANSCRIBE VOICE TO MIDI")
                                            .font(egui::FontId::proportional(14.0))
                                            .strong()
                                            .color(egui::Color32::BLACK)
                                    ).fill(egui::Color32::from_rgb(0, 230, 180)))
                                };

                                if transcribe_btn.clicked() {
                                    let samples = shared_state.recording_buffer.lock().clone();
                                    let model_dir_str = params.model_dir.lock().clone();
                                    if model_dir_str.is_empty() {
                                        *shared_state.status.lock() = "Error: Model path is not selected. Please click 'Browse...' and select the model folder first.".to_string();
                                    } else if samples.is_empty() {
                                        *shared_state.status.lock() = "Error: Recording buffer is empty. Record some audio or load a file first.".to_string();
                                    } else {
                                        shared_state.is_transcribing.store(true, Ordering::SeqCst);
                                        *shared_state.status.lock() = "Running GAME.rs inference engine...".to_string();

                                        let state_clone = shared_state.clone();
                                        let tempo = params.tempo.value();
                                        let seg_threshold = params.seg_threshold.value();
                                        let est_threshold = params.est_threshold.value();
                                        let nsteps = params.nsteps.value() as usize;
                                        let t0 = params.t0.value();
                                        let language_hint = params.language.value();
                                        let orig_sr = state_clone.sample_rate.load(Ordering::SeqCst);

                                        std::thread::spawn(move || {
                                            let res = run_transcription(
                                                samples,
                                                orig_sr,
                                                model_dir_str,
                                                tempo,
                                                seg_threshold,
                                                2, // seg_radius
                                                est_threshold,
                                                language_hint,
                                                nsteps,
                                                t0,
                                            );

                                            state_clone.is_transcribing.store(false, Ordering::SeqCst);
                                            match res {
                                                Ok(notes) => {
                                                    let count = notes.len();
                                                    *state_clone.transcribed_notes.lock() = Some(notes);
                                                    *state_clone.status.lock() = format!("Success! Transcribed {} notes.", count);
                                                }
                                                Err(e) => {
                                                    *state_clone.status.lock() = format!("Error: {}", e);
                                                }
                                            }
                                        });
                                    }
                                }

                                // Status display
                                let status_text = shared_state.status.lock().clone();
                                ui.label(
                                    egui::RichText::new(&status_text)
                                        .font(egui::FontId::proportional(11.0))
                                        .color(if status_text.starts_with("Error") {
                                            egui::Color32::from_rgb(255, 100, 100)
                                        } else if status_text.starts_with("Success") {
                                            egui::Color32::from_rgb(100, 255, 100)
                                        } else {
                                            egui::Color32::from_gray(180)
                                        })
                                );
                            });
                        });

                        ui.add_space(10.0);

                        // Piano Roll Grid and Export Group
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.heading(egui::RichText::new("🎹 Transcribed Notes").font(egui::FontId::proportional(14.0)).strong());
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let has_notes = shared_state.transcribed_notes.lock().is_some();
                                    if ui.add_enabled(has_notes, egui::Button::new("💾 Export MIDI")).clicked() {
                                        let notes_export = shared_state.transcribed_notes.lock().clone();
                                        let export_tempo = params.tempo.value();
                                        let state = shared_state.clone();
                                        std::thread::spawn(move || {
                                            let notes = match notes_export {
                                                Some(n) => n,
                                                None => return,
                                            };
                                            let path = match rfd::FileDialog::new()
                                                .add_filter("MIDI File", &["mid", "midi"])
                                                .set_file_name("transcription.mid")
                                                .save_file()
                                            {
                                                Some(p) => p,
                                                None => return,
                                            };
                                            match write_midi(&path, &notes, export_tempo) {
                                                Ok(_) => *state.status.lock() = format!("Saved: {:?}", path.file_name().unwrap()),
                                                Err(e) => *state.status.lock() = format!("Error: {}", e),
                                            }
                                        });
                                    }
                                    if ui.add_enabled(has_notes, egui::Button::new("🎵 Drag MIDI 🎵")).clicked() {
                                        if let Some(ref notes) = *shared_state.transcribed_notes.lock() {
                                            let tempo = params.tempo.value();
                                            let dir = std::env::temp_dir().join("copycat-midi");
                                            let _ = std::fs::create_dir_all(&dir);
                                            let path = dir.join("transcription.mid");
                                            match write_midi(&path, notes, tempo) {
                                                Ok(_) => {
                                                    *shared_state.status.lock() = format!("Drag from: {}", path.display());
                                                    #[cfg(target_os = "windows")]
                                                    drag_midi_file(&path);
                                                }
                                                Err(e) => *shared_state.status.lock() = format!("Error: {}", e),
                                            }
                                        }
                                    }
                                });
                            });

                            ui.add_space(5.0);

                            // Draw Piano Roll Widget
                            let notes_guard = shared_state.transcribed_notes.lock();
                            let notes = notes_guard.as_ref().map(|n| n.as_slice()).unwrap_or(&[]);
                            draw_piano_roll(ui, notes, 650.0, 140.0);
                        });
                    });
            },
        )
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let sample_rate = context.transport().sample_rate;
        self.shared_state.sample_rate.store(sample_rate as u32, Ordering::SeqCst);

        // 1. Record incoming audio
        let num_samples = buffer.samples();
        if self.shared_state.recording.load(Ordering::Relaxed) {
            if let Some(mut rec_buf) = self.shared_state.recording_buffer.try_lock() {
                let channel_slices = buffer.as_slice();
                let num_channels = channel_slices.len();
                if num_channels > 0 {
                    for frame in 0..num_samples {
                        let mut sum = 0.0;
                        for c in 0..num_channels {
                            sum += channel_slices[c][frame];
                        }
                        rec_buf.push(sum / num_channels as f32);
                    }
                }
            }
        }

        // 2. Playback of MIDI notes to DAW in real-time
        let transport = context.transport();
        if transport.playing {
            if let Some(pos_samples) = transport.pos_samples() {
                if let Some(notes_guard) = self.shared_state.transcribed_notes.try_lock() {
                    if let Some(ref notes) = *notes_guard {
                        let start_sec = pos_samples as f64 / sample_rate as f64;
                        let end_sec = (pos_samples + num_samples as i64) as f64 / sample_rate as f64;

                        for note in notes {
                            let onset_sec = note.onset as f64;
                            let offset_sec = note.offset as f64;
                            let pitch = note.pitch.round().clamp(0.0, 127.0) as u8;

                            // Emit Note On Event
                            if onset_sec >= start_sec && onset_sec < end_sec {
                                let offset_samples = ((onset_sec - start_sec) * sample_rate as f64).round() as u32;
                                context.send_event(NoteEvent::NoteOn {
                                    timing: offset_samples.min(num_samples as u32 - 1),
                                    voice_id: None,
                                    channel: 0,
                                    note: pitch,
                                    velocity: 0.8,
                                });
                            }

                            // Emit Note Off Event
                            if offset_sec >= start_sec && offset_sec < end_sec {
                                let offset_samples = ((offset_sec - start_sec) * sample_rate as f64).round() as u32;
                                context.send_event(NoteEvent::NoteOff {
                                    timing: offset_samples.min(num_samples as u32 - 1),
                                    voice_id: None,
                                    channel: 0,
                                    note: pitch,
                                    velocity: 0.0,
                                });
                            }
                        }
                    }
                }
            }
        }

        ProcessStatus::Normal
    }
}

// Waveform drawer widget
fn draw_waveform(ui: &mut egui::Ui, samples: &[f32], width: f32, height: f32) {
    if samples.is_empty() {
        let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(26, 26, 30));
        ui.painter().line_segment(
            [egui::pos2(rect.left(), rect.center().y), egui::pos2(rect.right(), rect.center().y)],
            egui::Stroke::new(1.0, egui::Color32::from_rgb(50, 50, 60)),
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No audio recorded. Record from DAW or load file.",
            egui::FontId::proportional(12.0),
            egui::Color32::from_gray(100),
        );
        return;
    }

    let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(22, 22, 26));

    let stroke = egui::Stroke::new(1.2, egui::Color32::from_rgb(0, 230, 180));
    let center_y = rect.center().y;
    let num_bins = width.round() as usize;
    if num_bins == 0 {
        return;
    }

    let chunk_size = (samples.len() as f32 / num_bins as f32).max(1.0) as usize;

    for i in 0..num_bins {
        let start = i * chunk_size;
        let end = ((i + 1) * chunk_size).min(samples.len());
        if start >= samples.len() {
            break;
        }

        let chunk = &samples[start..end];
        let mut max_val = 0.0f32;
        let mut min_val = 0.0f32;
        for &s in chunk {
            if s > max_val {
                max_val = s;
            }
            if s < min_val {
                min_val = s;
            }
        }

        let x = rect.left() + i as f32;
        let y_max = center_y - max_val.clamp(0.0, 1.0) * (height * 0.45);
        let y_min = center_y - min_val.clamp(-1.0, 0.0) * (height * 0.45);

        ui.painter().line_segment([egui::pos2(x, y_min), egui::pos2(x, y_max)], stroke);
    }
}

// Piano roll notes drawer widget
fn draw_piano_roll(ui: &mut egui::Ui, notes: &[NoteInfo], width: f32, height: f32) {
    if notes.is_empty() {
        let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(26, 26, 30));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No transcribed notes. Click Transcribe to process.",
            egui::FontId::proportional(12.0),
            egui::Color32::from_gray(100),
        );
        return;
    }

    let (rect, _response) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(22, 22, 26));

    let max_duration = notes.iter().map(|n| n.offset).fold(0.0f32, |a, b| a.max(b));
    let mut min_pitch = notes.iter().map(|n| n.pitch.round() as i32).min().unwrap_or(60);
    let mut max_pitch = notes.iter().map(|n| n.pitch.round() as i32).max().unwrap_or(72);

    min_pitch = (min_pitch - 2).max(0);
    max_pitch = (max_pitch + 2).min(127);
    let pitch_range = (max_pitch - min_pitch).max(1) as f32;

    // Draw horizontal grid lines
    let grid_color = egui::Color32::from_rgb(40, 40, 48);
    for p in min_pitch..=max_pitch {
        let y = rect.top() + ((max_pitch - p) as f32 / pitch_range) * rect.height();
        ui.painter().line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(0.5, grid_color),
        );
    }

    // Draw notes
    let note_color = egui::Color32::from_rgb(160, 90, 240);
    let note_border = egui::Color32::from_rgb(210, 170, 255);
    let note_stroke = egui::Stroke::new(1.0, note_border);

    for note in notes {
        if max_duration <= 0.0 {
            continue;
        }
        let left = rect.left() + (note.onset / max_duration) * rect.width();
        let right = rect.left() + (note.offset / max_duration) * rect.width();

        let pitch_round = note.pitch.round() as i32;
        let y_top = rect.top() + ((max_pitch - pitch_round) as f32 / pitch_range) * rect.height();
        let y_bottom = rect.top() + ((max_pitch - pitch_round + 1) as f32 / pitch_range) * rect.height();

        let note_rect = egui::Rect::from_min_max(
            egui::pos2(left + 1.0, y_top + 1.0),
            egui::pos2(right - 1.0, y_bottom - 1.0),
        );

        ui.painter().rect(note_rect, 2.0, note_color, note_stroke, egui::epaint::StrokeKind::Inside);

        if note_rect.width() > 30.0 {
            let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
            let name = note_names[(pitch_round % 12) as usize];
            let octave = (pitch_round / 12) - 1;
            let note_str = format!("{}{}", name, octave);
            ui.painter().text(
                note_rect.center(),
                egui::Align2::CENTER_CENTER,
                note_str,
                egui::FontId::proportional(9.0),
                egui::Color32::WHITE,
            );
        }
    }
}

// Resampler logic
fn resample(samples: &[f32], from_sr: u32, to_sr: u32) -> Vec<f32> {
    if from_sr == to_sr {
        return samples.to_vec();
    }
    let ratio = to_sr as f64 / from_sr as f64;
    let new_len = (samples.len() as f64 * ratio).round() as usize;
    let mut resampled = Vec::with_capacity(new_len);
    for i in 0..new_len {
        let pos = i as f64 / ratio;
        let idx = pos.floor() as usize;
        let frac = pos - idx as f64;
        if idx + 1 < samples.len() {
            let s0 = samples[idx];
            let s1 = samples[idx + 1];
            resampled.push(s0 + frac as f32 * (s1 - s0));
        } else if idx < samples.len() {
            resampled.push(samples[idx]);
        }
    }
    resampled
}

// Symphonia Audio Loader
fn load_audio(path: &std::path::Path) -> anyhow::Result<(Vec<f32>, u32)> {
    let src = std::fs::File::open(path)?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = symphonia::core::probe::Hint::new();
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        hint.with_extension(ext);
    }

    let meta_opts = symphonia::core::meta::MetadataOptions::default();
    let fmt_opts = symphonia::core::formats::FormatOptions::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or_else(|| anyhow::anyhow!("no audio track found"))?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);

    let dec_opts = symphonia::core::codecs::DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &dec_opts)?;

    let mut samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(ref err)) if err.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(err) => return Err(err.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                let num_channels = decoded.spec().channels.count();
                let num_frames = decoded.frames();

                let spec = *decoded.spec();
                let mut sample_buf = symphonia::core::audio::SampleBuffer::<f32>::new(num_frames as u64, spec);
                sample_buf.copy_interleaved_ref(decoded);
                let interleaved_samples = sample_buf.samples();

                for f in 0..num_frames {
                    let mut sum = 0.0;
                    for c in 0..num_channels {
                        sum += interleaved_samples[f * num_channels + c];
                    }
                    samples.push(sum / num_channels as f32);
                }
            }
            Err(symphonia::core::errors::Error::DecodeError(err)) => {
                eprintln!("decode error: {}", err);
            }
            Err(err) => return Err(err.into()),
        }
    }

    Ok((samples, sample_rate))
}

// MIDI Exporter
fn write_midi(path: &std::path::Path, notes: &[NoteInfo], tempo_bpm: f32) -> anyhow::Result<()> {
    use midly::{Header, Smf, Track, TrackEvent, TrackEventKind, MidiMessage, Timing, Format};
    use midly::num::u24;

    let header = Header::new(Format::SingleTrack, Timing::Metrical(480.into()));
    let mut track = Track::new();

    let mpb = (60_000_000.0 / tempo_bpm).round() as u32;
    track.push(TrackEvent {
        delta: 0.into(),
        kind: TrackEventKind::Meta(midly::MetaMessage::Tempo(u24::from_int_lossy(mpb))),
    });

    let mut last_time_ticks = 0u32;
    for note in notes {
        let onset_ticks = (note.onset * tempo_bpm * 8.0).round() as u32;
        let offset_ticks = (note.offset * tempo_bpm * 8.0).round() as u32;
        let midi_pitch = note.pitch.round().clamp(0.0, 127.0) as u8;

        if offset_ticks <= onset_ticks {
            continue;
        }

        let delta_on = onset_ticks.checked_sub(last_time_ticks).unwrap_or(0);
        track.push(TrackEvent {
            delta: delta_on.into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: MidiMessage::NoteOn {
                    key: midi_pitch.into(),
                    vel: 127.into(),
                },
            },
        });

        let delta_off = offset_ticks - onset_ticks;
        track.push(TrackEvent {
            delta: delta_off.into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: MidiMessage::NoteOff {
                    key: midi_pitch.into(),
                    vel: 0.into(),
                },
            },
        });

        last_time_ticks = offset_ticks;
    }

        let smf = Smf {
        header,
        tracks: vec![track],
    };

    smf.save(path)?;
    Ok(())
}

// OLE drag-and-drop: initiate a system drag with a MIDI file
#[cfg(target_os = "windows")]
fn drag_midi_file(path: &std::path::Path) {
    // Check if running under Wine by testing if ntdll has wine_get_version
    use std::ffi::c_void;
    use std::ptr::null_mut;

    // COM interfaces
    #[repr(C)]
    struct IDataObjectVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const c_void, *mut *mut c_void) -> i32,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        get_data: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void) -> i32,
        get_data_here: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void) -> i32,
        query_get_data: unsafe extern "system" fn(*mut c_void, *const c_void) -> i32,
        get_canonical_format_etc: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void) -> i32,
        set_data: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void, i32) -> i32,
        enum_format_etc: unsafe extern "system" fn(*mut c_void, u32, *mut *mut c_void) -> i32,
        dadvise: unsafe extern "system" fn(*mut c_void, *const c_void, u32, *mut c_void, *mut u32) -> i32,
        dunadvise: unsafe extern "system" fn(*mut c_void, u32) -> i32,
        enum_dadvise: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    }
    #[repr(C)]
    struct DataObj { vtbl: *const IDataObjectVtbl, refcount: u32, path_wide: *mut u16 }

    #[repr(C)]
    struct IDropSourceVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const c_void, *mut *mut c_void) -> i32,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        query_continue_drag: unsafe extern "system" fn(*mut c_void, i32, u32, *mut u32) -> i32,
        give_feedback: unsafe extern "system" fn(*mut c_void, u32) -> i32,
    }
    #[repr(C)]
    struct DropSrc { vtbl: *const IDropSourceVtbl, refcount: u32 }

    // IEnumFORMATETC provides a single CF_HDROP format
    #[repr(C)]
    struct IEnumFmtVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const c_void, *mut *mut c_void) -> i32,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        next_fn: unsafe extern "system" fn(*mut c_void, u32, *mut c_void, *mut u32) -> i32,
        skip: unsafe extern "system" fn(*mut c_void, u32) -> i32,
        reset: unsafe extern "system" fn(*mut c_void) -> i32,
        clone_fn: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> i32,
    }
    #[repr(C)]
    struct EnumFmt { vtbl: *const IEnumFmtVtbl, refcount: u32, pos: u32, count: u32 }

    const GHND: u32 = 0x0042;
    const CF_HDROP: u32 = 15;
    const S_OK: i32 = 0;
    const S_FALSE: i32 = 1;
    const E_UNEXPECTED: i32 = -2147418113i32; // 0x8000FFFF
    const DVASPECT_CONTENT: u32 = 1;
    const TYMED_HGLOBAL: u32 = 1;
    const DRAGDROP_S_DROP: u32 = 0x00040100;
    const DRAGDROP_S_CANCEL: u32 = 0x00040101;
    const E_FAIL: i32 = -2147467259i32; // 0x80004005
    const E_NOTIMPL: i32 = -2147467263i32; // 0x80004001
    const DV_E_FORMATETC: i32 = -2147221404i32; // 0x80040064
    const MK_LBUTTON: i32 = 1;
    const DROPEFFECT_COPY: u32 = 1;
    const FORMATETC: [u32; 2] = [CF_HDROP, 0]; // cf, ptd, dwAspect, lindex, tymed
    // Actually FORMATETC is larger, let me use bytes

    extern "system" {
        fn DoDragDrop(
            pDataObj: *mut c_void,
            pDropSource: *mut c_void,
            dwOKEffect: u32,
            pdwEffect: *mut u32,
        ) -> i32;
        fn OleInitialize(pvReserved: *mut c_void) -> i32;
        fn GlobalAlloc(uFlags: u32, dwBytes: usize) -> *mut c_void;
        fn GlobalLock(hMem: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(hMem: *mut c_void) -> i32;
        fn GlobalFree(hMem: *mut c_void) -> *mut c_void;
    }

    unsafe {
        OleInitialize(null_mut());

        let wide: Vec<u16> = path.to_string_lossy()
            .encode_utf16().chain(std::iter::once(0)).collect();
        let wide_ptr = wide.as_ptr() as *mut u16;

    // IUnknown for IDataObject
    unsafe extern "system" fn do_query_interface(_this: *mut c_void, _riid: *const c_void, ppv: *mut *mut c_void) -> i32 {
        *ppv = _this;
        let obj = _this as *mut DataObj;
        let add_ref = (*(*obj).vtbl).add_ref;
        add_ref(_this);
        S_OK
    }
    unsafe extern "system" fn do_add_ref(this: *mut c_void) -> u32 {
        let obj = this as *mut DataObj;
        (*obj).refcount += 1;
        (*obj).refcount
    }
    unsafe extern "system" fn do_release(this: *mut c_void) -> u32 {
        let obj = this as *mut DataObj;
        (*obj).refcount -= 1;
        if (*obj).refcount == 0 {
            let _ = Vec::from_raw_parts((*obj).path_wide, 0, 0);
            let _ = Box::from_raw(this as *mut DataObj);
            0
        } else {
            (*obj).refcount
        }
    }
    unsafe extern "system" fn do_get_data(
        this: *mut c_void, pformatetc: *const c_void, pmedium: *mut c_void,
    ) -> i32 {
        let fmt = *(pformatetc as *const u32);
        if fmt != CF_HDROP { return S_FALSE; }

        let obj = this as *mut DataObj;
        let wide_len = {
            let mut len = 0;
            while *((*obj).path_wide.add(len)) != 0 { len += 1; }
            len
        };

        #[repr(C)]
        struct DROPFILES { pFiles: u32, pt: (i32,i32), fNC: i32, fWide: i32 }
        let drop_size = std::mem::size_of::<DROPFILES>();
        let total = drop_size + (wide_len as usize + 1) * 2;

        let h = GlobalAlloc(GHND, total);
        if h.is_null() { return E_UNEXPECTED; }
        let ptr = GlobalLock(h) as *mut u8;
        if ptr.is_null() { GlobalFree(h); return E_UNEXPECTED; }

        (ptr as *mut DROPFILES).write(DROPFILES {
            pFiles: drop_size as u32, pt: (0,0), fNC: 0, fWide: 1,
        });
        std::ptr::copy_nonoverlapping((*obj).path_wide, ptr.add(drop_size) as *mut u16, wide_len + 1);
        GlobalUnlock(h);

        // STGMEDIUM: tymed(u32), pad, hGlobal(ptr), pUnkForRelease(ptr)
        *(pmedium as *mut u32) = TYMED_HGLOBAL;
        *(pmedium.add(8) as *mut *mut c_void) = h as *mut c_void;
        S_OK
    }
    // Stub implementations for other IDataObject methods
    unsafe extern "system" fn do_get_data_here(_: *mut c_void, _: *const c_void, _: *mut c_void) -> i32 { S_FALSE }
    unsafe extern "system" fn do_query_get_data(_: *mut c_void, _: *const c_void) -> i32 { S_FALSE }
    unsafe extern "system" fn do_get_canonical(_: *mut c_void, _: *const c_void, _: *mut c_void) -> i32 { E_NOTIMPL }
    unsafe extern "system" fn do_set_data(_: *mut c_void, _: *const c_void, _: *mut c_void, _: i32) -> i32 { S_FALSE }
    unsafe extern "system" fn do_enum_fmt(this: *mut c_void, _direction: u32, ppenum: *mut *mut c_void) -> i32 {
        // IEnumFORMATETC vtbl that provides our CF_HDROP format
        unsafe extern "system" fn ef_query_interface(_t: *mut c_void, _r: *const c_void, ppv: *mut *mut c_void) -> i32 {
            *ppv = _t;
            let e = _t as *mut EnumFmt;
            let ar = (*(*e).vtbl).add_ref;
            ar(_t);
            S_OK
        }
        unsafe extern "system" fn ef_add_ref(t: *mut c_void) -> u32 { let o = t as *mut EnumFmt; (*o).refcount += 1; (*o).refcount }
        unsafe extern "system" fn ef_release(t: *mut c_void) -> u32 {
            let o = t as *mut EnumFmt; (*o).refcount -= 1;
            if (*o).refcount == 0 { let _ = Box::from_raw(t as *mut EnumFmt); 0 } else { (*o).refcount }
        }
        unsafe extern "system" fn ef_next(t: *mut c_void, celt: u32, rgelt: *mut c_void, pceltFetched: *mut u32) -> i32 {
            let o = t as *mut EnumFmt;
            if (*o).pos >= (*o).count || celt == 0 { if !pceltFetched.is_null() { *pceltFetched = 0; } return S_FALSE; }
            // FORMATETC layout: cfFormat(u16)+pad(u16)=u32, ptd(u64), dwAspect(u32), lindex(i32), tymed(u32) = 24 bytes
            let fe = rgelt as *mut u32;
            *fe = CF_HDROP;  // cfFormat = 15, ptd = 0 (upper bits zero from alloc)
            *(fe.add(1)) = 0; // ptd = null
            *(fe.add(2)) = 0; // ptd upper 32 bits
            *(fe.add(3)) = DVASPECT_CONTENT; // dwAspect
            *(fe.add(4)) = -1i32 as u32; // lindex = -1
            *(fe.add(5)) = TYMED_HGLOBAL; // tymed
            (*o).pos += 1;
            if !pceltFetched.is_null() { *pceltFetched = 1; }
            S_OK
        }
        unsafe extern "system" fn ef_skip(t: *mut c_void, celt: u32) -> i32 {
            let o = t as *mut EnumFmt;
            if (*o).pos + celt <= (*o).count { (*o).pos += celt; S_OK } else { S_FALSE }
        }
        unsafe extern "system" fn ef_reset(t: *mut c_void) -> i32 { (*(t as *mut EnumFmt)).pos = 0; S_OK }
        unsafe extern "system" fn ef_clone(t: *mut c_void, ppenum: *mut *mut c_void) -> i32 {
            let src = t as *mut EnumFmt;
            let c = Box::into_raw(Box::new(EnumFmt { vtbl: (*src).vtbl, refcount: 1, pos: (*src).pos, count: (*src).count }));
            *ppenum = c as *mut c_void;
            S_OK
        }

        let vtbl = Box::into_raw(Box::new(IEnumFmtVtbl {
            query_interface: ef_query_interface, add_ref: ef_add_ref, release: ef_release,
            next_fn: ef_next, skip: ef_skip, reset: ef_reset, clone_fn: ef_clone,
        }));
        let enum_fmt = Box::into_raw(Box::new(EnumFmt { vtbl: vtbl as *const IEnumFmtVtbl, refcount: 1, pos: 0, count: 1 }));
        *ppenum = enum_fmt as *mut c_void;
        S_OK
    }
    unsafe extern "system" fn do_dadvise(_: *mut c_void, _: *const c_void, _: u32, _: *mut c_void, _: *mut u32) -> i32 { S_FALSE }
        unsafe extern "system" fn do_dunadvise(_: *mut c_void, _: u32) -> i32 { S_FALSE }
        unsafe extern "system" fn do_enum_dadvise(_: *mut c_void, _: *mut *mut c_void) -> i32 { S_FALSE }

        let ido = Box::into_raw(Box::new(IDataObjectVtbl {
            query_interface: do_query_interface,
            add_ref: do_add_ref,
            release: do_release,
            get_data: do_get_data,
            get_data_here: do_get_data_here,
            query_get_data: do_query_get_data,
            get_canonical_format_etc: do_get_canonical,
            set_data: do_set_data,
            enum_format_etc: do_enum_fmt,
            dadvise: do_dadvise,
            dunadvise: do_dunadvise,
            enum_dadvise: do_enum_dadvise,
        }));
        let ido_vtbl: *const IDataObjectVtbl = ido as *const IDataObjectVtbl;

        let data_obj = Box::into_raw(Box::new(DataObj {
            vtbl: ido_vtbl,
            refcount: 1,
            path_wide: wide_ptr,
        }));

        // IDropSource stubs
        unsafe extern "system" fn ds_query_interface(
            this: *mut c_void, _: *const c_void, ppv: *mut *mut c_void,
        ) -> i32 { *ppv = this; let obj = this as *mut DropSrc; let ar = (*(*obj).vtbl).add_ref; ar(this); S_OK }
        unsafe extern "system" fn ds_add_ref(this: *mut c_void) -> u32 { (*(this as *mut DropSrc)).refcount += 1; (*(this as *mut DropSrc)).refcount }
        unsafe extern "system" fn ds_release(this: *mut c_void) -> u32 {
            let obj = this as *mut DropSrc;
            (*obj).refcount -= 1;
            if (*obj).refcount == 0 { let _ = Box::from_raw(this as *mut DropSrc); }
            (*obj).refcount
        }
        unsafe extern "system" fn ds_continue_drag(
            _: *mut c_void, fEscapePressed: i32, _: u32, pdwEffect: *mut u32,
        ) -> i32 {
            if fEscapePressed != 0 { *pdwEffect = 0; return DRAGDROP_S_CANCEL as i32; }
            *pdwEffect = DROPEFFECT_COPY;
            S_OK
        }
        unsafe extern "system" fn ds_give_feedback(_: *mut c_void, _: u32) -> i32 { S_OK }

        let ds_vtbl = Box::into_raw(Box::new(IDropSourceVtbl {
            query_interface: ds_query_interface,
            add_ref: ds_add_ref,
            release: ds_release,
            query_continue_drag: ds_continue_drag,
            give_feedback: ds_give_feedback,
        })) as *const IDropSourceVtbl;

        let drop_src = Box::into_raw(Box::new(DropSrc { vtbl: ds_vtbl, refcount: 1 }));

        let mut effect: u32 = 0;
        DoDragDrop(data_obj as *mut c_void, drop_src as *mut c_void, DROPEFFECT_COPY, &mut effect);

        // Cleanup
        do_release(data_obj as *mut c_void);
        ds_release(drop_src as *mut c_void);
    }
}

// Background ONNX transcription runner
fn run_transcription(
    samples: Vec<f32>,
    orig_sr: u32,
    model_dir_str: String,
    _tempo: f32,
    seg_threshold: f32,
    seg_radius: i64,
    est_threshold: f32,
    language_hint: i32,
    nsteps: usize,
    t0: f32,
) -> Result<Vec<NoteInfo>, String> {
    let model_dir = std::path::PathBuf::from(&model_dir_str);
    let _ = ort::init().commit();
    let config_path = model_dir.join("config.json");
    if !config_path.exists() {
        return Err(format!("Model config file not found: {:?}", config_path));
    }

    let config_data = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config.json: {}", e))?;
    
    #[derive(serde::Deserialize, Debug)]
    #[allow(dead_code)]
    struct ModelConfig {
        samplerate: u32,
        timestep: f32,
        #[serde(alias = "loop")]
        loop_enabled: Option<bool>,
        embedding_dim: usize,
    }
    
    let config: ModelConfig = serde_json::from_str(&config_data)
        .map_err(|e| format!("Failed to parse config.json: {}", e))?;

    let resampled = resample(&samples, orig_sr, config.samplerate);
    let total_samples = resampled.len();
    let audio_duration = total_samples as f32 / config.samplerate as f32;

    let encoder_sess = Session::builder()
        .map_err(|e| format!("Failed to build encoder session builder: {}", e))?
        .commit_from_file(model_dir.join("encoder.onnx"))
        .map_err(|e| format!("Failed to load encoder.onnx: {}. Check model path.", e))?;

    let segmenter_sess = Session::builder()
        .map_err(|e| format!("Failed to build segmenter session builder: {}", e))?
        .commit_from_file(model_dir.join("segmenter.onnx"))
        .map_err(|e| format!("Failed to load segmenter.onnx: {}", e))?;

    let bd2dur_sess = Session::builder()
        .map_err(|e| format!("Failed to build bd2dur session builder: {}", e))?
        .commit_from_file(model_dir.join("bd2dur.onnx"))
        .map_err(|e| format!("Failed to load bd2dur.onnx: {}", e))?;

    let estimator_sess = Session::builder()
        .map_err(|e| format!("Failed to build estimator session builder: {}", e))?
        .commit_from_file(model_dir.join("estimator.onnx"))
        .map_err(|e| format!("Failed to load estimator.onnx: {}", e))?;

    let waveform_arr = Array2::from_shape_vec((1, total_samples), resampled)
        .map_err(|e| format!("Failed to create waveform array: {}", e))?;
    let duration_arr = Array1::from_vec(vec![audio_duration]);

    let waveform_val = Tensor::from_array(waveform_arr)
        .map_err(|e| format!("Failed to create waveform tensor: {}", e))?;
    let duration_val = Tensor::from_array(duration_arr)
        .map_err(|e| format!("Failed to create duration tensor: {}", e))?;

    let encoder_outputs = encoder_sess.run(vec![
        (Cow::from("waveform"), SessionInputValue::from(waveform_val)),
        (Cow::from("duration"), SessionInputValue::from(duration_val)),
    ]).map_err(|e| format!("Encoder run failed: {}", e))?;

    let x_seg_view = encoder_outputs[0].try_extract_tensor::<f32>()
        .map_err(|e| format!("Failed to extract x_seg: {}", e))?;
    let x_est_view = encoder_outputs[1].try_extract_tensor::<f32>()
        .map_err(|e| format!("Failed to extract x_est: {}", e))?;
    let mask_t_view = encoder_outputs[2].try_extract_tensor::<bool>()
        .map_err(|e| format!("Failed to extract mask_t: {}", e))?;

    let num_frames = mask_t_view.shape()[1] as usize;

    let x_seg_tensor = Tensor::from_array(x_seg_view.to_owned())
        .map_err(|e| format!("Failed to create x_seg tensor: {}", e))?;
    let x_est_tensor = Tensor::from_array(x_est_view.to_owned())
        .map_err(|e| format!("Failed to create x_est tensor: {}", e))?;
    let mask_t_tensor = Tensor::from_array(mask_t_view.to_owned())
        .map_err(|e| format!("Failed to create mask_t tensor: {}", e))?;

    let step = (1.0 - t0) / nsteps as f32;
    let ts: Vec<f32> = (0..nsteps)
        .map(|i| t0 + i as f32 * step)
        .collect();

    let mut boundaries_val = Tensor::from_array(Array2::<bool>::from_elem((1, num_frames), false))
        .map_err(|e| format!("Failed to create boundaries tensor: {}", e))?;
    let known_boundaries_val = Tensor::from_array(Array2::<bool>::from_elem((1, num_frames), false))
        .map_err(|e| format!("Failed to create known boundaries tensor: {}", e))?;

    let language_val = Tensor::from_array(Array1::from_vec(vec![language_hint as i64]))
        .map_err(|e| format!("Failed to create language tensor: {}", e))?;
    let seg_threshold_val = Tensor::from_array(Array0::from_elem((), seg_threshold))
        .map_err(|e| format!("Failed to create seg_threshold tensor: {}", e))?;
    let seg_radius_val = Tensor::from_array(Array0::from_elem((), seg_radius))
        .map_err(|e| format!("Failed to create seg_radius tensor: {}", e))?;

    for &t_val in &ts {
        let t_tensor = Tensor::from_array(Array1::from_vec(vec![t_val]))
            .map_err(|e| format!("Failed to create t tensor: {}", e))?;
        let outputs = segmenter_sess.run(vec![
            (Cow::from("x_seg"), SessionInputValue::from(x_seg_tensor.view())),
            (Cow::from("language"), SessionInputValue::from(language_val.view())),
            (Cow::from("known_boundaries"), SessionInputValue::from(known_boundaries_val.view())),
            (Cow::from("prev_boundaries"), SessionInputValue::from(boundaries_val.view())),
            (Cow::from("t"), SessionInputValue::from(t_tensor.view())),
            (Cow::from("maskT"), SessionInputValue::from(mask_t_tensor.view())),
            (Cow::from("threshold"), SessionInputValue::from(seg_threshold_val.view())),
            (Cow::from("radius"), SessionInputValue::from(seg_radius_val.view())),
        ]).map_err(|e| format!("Segmenter run failed: {}", e))?;

        let boundaries_view = outputs[0].try_extract_tensor::<bool>()
            .map_err(|e| format!("Failed to extract boundaries: {}", e))?;
        boundaries_val = Tensor::from_array(boundaries_view.to_owned())
            .map_err(|e| format!("Failed to recreate boundaries tensor: {}", e))?;
    }

    let bd2dur_outputs = bd2dur_sess.run(vec![
        (Cow::from("boundaries"), SessionInputValue::from(boundaries_val.view())),
        (Cow::from("maskT"), SessionInputValue::from(mask_t_tensor.view())),
    ]).map_err(|e| format!("bd2dur run failed: {}", e))?;

    let durations_view = bd2dur_outputs[0].try_extract_tensor::<f32>()
        .map_err(|e| format!("Failed to extract durations: {}", e))?;
    let mask_n_view = bd2dur_outputs[1].try_extract_tensor::<bool>()
        .map_err(|e| format!("Failed to extract note mask: {}", e))?;

    let mask_n_tensor = Tensor::from_array(mask_n_view.to_owned())
        .map_err(|e| format!("Failed to create mask_n tensor: {}", e))?;

    let est_threshold_val = Tensor::from_array(Array0::from_elem((), est_threshold))
        .map_err(|e| format!("Failed to create est_threshold tensor: {}", e))?;

    let estimator_outputs = estimator_sess.run(vec![
        (Cow::from("x_est"), SessionInputValue::from(x_est_tensor.view())),
        (Cow::from("boundaries"), SessionInputValue::from(boundaries_val.view())),
        (Cow::from("maskT"), SessionInputValue::from(mask_t_tensor.view())),
        (Cow::from("maskN"), SessionInputValue::from(mask_n_tensor.view())),
        (Cow::from("threshold"), SessionInputValue::from(est_threshold_val.view())),
    ]).map_err(|e| format!("Estimator run failed: {}", e))?;

    let presence_view = estimator_outputs[0].try_extract_tensor::<bool>()
        .map_err(|e| format!("Failed to extract presence data: {}", e))?;
    let scores_view = estimator_outputs[1].try_extract_tensor::<f32>()
        .map_err(|e| format!("Failed to extract scores data: {}", e))?;

    let presence_slice = presence_view.as_slice().ok_or("presence data is not contiguous")?;
    let scores_slice = scores_view.as_slice().ok_or("scores data is not contiguous")?;
    let durations_slice = durations_view.as_slice().ok_or("durations data is not contiguous")?;

    let mut notes = Vec::new();
    let mut current_time = 0.0;

    for i in 0..presence_slice.len() {
        let dur = durations_slice[i];
        let onset = current_time;
        let offset = current_time + dur;
        current_time = offset;

        let valid = presence_slice[i];
        let pitch = scores_slice[i];

        if offset - onset <= 0.0 {
            continue;
        }
        if !valid {
            continue;
        }

        notes.push(NoteInfo {
            onset,
            offset,
            pitch,
        });
    }

    notes.sort_by(|a, b| {
        a.onset.partial_cmp(&b.onset).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.offset.partial_cmp(&b.offset).unwrap_or(std::cmp::Ordering::Equal))
            .then(a.pitch.partial_cmp(&b.pitch).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut clean_notes = Vec::new();
    let mut last_time = 0.0;
    for mut note in notes {
        note.onset = note.onset.max(last_time);
        note.offset = note.offset.max(note.onset);
        if note.offset > note.onset {
            last_time = note.offset;
            clean_notes.push(note);
        }
    }

    Ok(clean_notes)
}

impl Vst3Plugin for Copycat {
    const VST3_CLASS_ID: [u8; 16] = *b"CopycatVoice2Mid";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Fx];
}

impl ClapPlugin for Copycat {
    const CLAP_ID: &'static str = "org.openvpi.copycat";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Voice-to-MIDI VST3/CLAP Plugin using GAME AI");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::NoteDetector];
}

nih_export_vst3!(Copycat);
nih_export_clap!(Copycat);
