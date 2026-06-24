use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;
use eframe::egui;

const CHECKPOINT_URL: &str = "https://github.com/openvpi/GAME/releases/download/v1.0.3/GAME-1.0.3-large-onnx.zip";

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn install_plugin_files(src_dir: &Path, status: &Arc<Mutex<String>>) -> anyhow::Result<()> {
    let clap_src = src_dir.join("copycat.clap");
    let vst3_src = src_dir.join("copycat.vst3");
    #[cfg(target_os = "windows")]
    let dll_src = src_dir.join("onnxruntime.dll");

    let mut clap_installed = false;
    let mut vst3_installed = false;

    // Determine target folders
    #[cfg(target_os = "windows")]
    {
        let common_files = std::env::var("COMMONPROGRAMFILES")
            .unwrap_or_else(|_| "C:\\Program Files\\Common Files".to_string());
        let local_app_data = std::env::var("LOCALAPPDATA")
            .ok()
            .or_else(|| {
                std::env::var("USERPROFILE").ok().map(|p| {
                    Path::new(&p).join("AppData").join("Local").to_string_lossy().to_string()
                })
            });

        // 1. CLAP Installation
        if clap_src.exists() {
            let sys_clap_dir = PathBuf::from(&common_files).join("CLAP");
            let user_clap_dir = local_app_data.as_ref().map(|lad| PathBuf::from(lad).join("Programs").join("Common").join("CLAP"));

            *status.lock() = "Installing CLAP plugin...".to_string();
            let mut success = false;
            let mut errors = Vec::new();

            let target_sys = sys_clap_dir.join("copycat.clap");
            if fs::create_dir_all(&sys_clap_dir).is_ok() && fs::copy(&clap_src, &target_sys).is_ok() {
                if dll_src.exists() {
                    let _ = fs::copy(&dll_src, sys_clap_dir.join("onnxruntime.dll"));
                }
                *status.lock() = format!("CLAP installed to system CLAP directory.");
                success = true;
            } else {
                errors.push("System directory write failed (Admin permissions may be needed)");
            }

            // Try user local path if system path failed
            if !success {
                if let Some(ref ucd) = user_clap_dir {
                    let target_user = ucd.join("copycat.clap");
                    if fs::create_dir_all(ucd).is_ok() && fs::copy(&clap_src, &target_user).is_ok() {
                        if dll_src.exists() {
                            let _ = fs::copy(&dll_src, ucd.join("onnxruntime.dll"));
                        }
                        *status.lock() = format!("CLAP installed to user CLAP directory.");
                        success = true;
                    } else {
                        errors.push("User directory write failed");
                    }
                }
            }

            if success {
                clap_installed = true;
            } else {
                anyhow::bail!("Failed to install CLAP. Try running the installer as Administrator.\nErrors: {:?}", errors);
            }
        }

        // 2. VST3 Installation
        if vst3_src.exists() && vst3_src.is_dir() {
            let sys_vst3_dir = PathBuf::from(&common_files).join("VST3");
            let user_vst3_dir = local_app_data.as_ref().map(|lad| PathBuf::from(lad).join("Programs").join("Common").join("VST3"));

            *status.lock() = "Installing VST3 plugin...".to_string();
            let mut success = false;
            let mut errors = Vec::new();

            let target_sys = sys_vst3_dir.join("copycat.vst3");
            let _ = fs::remove_dir_all(&target_sys); // clean old
            if copy_dir_all(&vst3_src, &target_sys).is_ok() {
                *status.lock() = format!("VST3 installed to system VST3 directory.");
                success = true;
            } else {
                errors.push("System directory write failed (Admin permissions may be needed)");
            }

            if !success {
                if let Some(ref uvd) = user_vst3_dir {
                    let target_user = uvd.join("copycat.vst3");
                    let _ = fs::remove_dir_all(&target_user); // clean old
                    if copy_dir_all(&vst3_src, &target_user).is_ok() {
                        *status.lock() = format!("VST3 installed to user VST3 directory.");
                        success = true;
                    } else {
                        errors.push("User directory write failed");
                    }
                }
            }

            if success {
                vst3_installed = true;
            } else {
                anyhow::bail!("Failed to install VST3. Try running the installer as Administrator.\nErrors: {:?}", errors);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let home = std::env::var("HOME")?;
        let home_path = Path::new(&home);

        // 1. CLAP Installation
        if clap_src.exists() {
            let clap_dir = home_path.join(".clap");
            let target = clap_dir.join("copycat.clap");
            *status.lock() = "Installing CLAP to ~/.clap/".to_string();
            fs::create_dir_all(&clap_dir)?;
            fs::copy(&clap_src, &target)?;
            clap_installed = true;
        }

        // 2. VST3 Installation
        if vst3_src.exists() && vst3_src.is_dir() {
            let vst3_dir = home_path.join(".vst3");
            let target = vst3_dir.join("copycat.vst3");
            *status.lock() = "Installing VST3 to ~/.vst3/".to_string();
            let _ = fs::remove_dir_all(&target);
            copy_dir_all(&vst3_src, &target)?;
            vst3_installed = true;
        }
    }

    if !clap_installed && !vst3_installed {
        anyhow::bail!("No plugin binaries (copycat.clap / copycat.vst3) found to install.");
    }

    Ok(())
}

fn download_and_extract_model(
    model_install_path: &str,
    status: &Arc<Mutex<String>>,
    download_progress: &Arc<Mutex<f32>>,
    extract_progress: &Arc<Mutex<f32>>,
) -> anyhow::Result<()> {
    let models_base_dir = Path::new(model_install_path);
    fs::create_dir_all(models_base_dir)?;
    let zip_path = models_base_dir.join("download.zip");

    *status.lock() = "Connecting to download server...".to_string();
    let response = ureq::get(CHECKPOINT_URL).call()?;
    
    let total_size = response
        .header("Content-Length")
        .and_then(|len| len.parse::<usize>().ok());

    let mut file = fs::File::create(&zip_path)?;
    let mut reader = response.into_reader();
    let mut buffer = [0; 65536];
    let mut downloaded = 0;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read;

        if let Some(total) = total_size {
            let val = downloaded as f32 / total as f32;
            *download_progress.lock() = val;
            *status.lock() = format!("Downloading: {:.1}% ({:.1} / {:.1} MB)", val * 100.0, downloaded as f32 / 1024.0 / 1024.0, total as f32 / 1024.0 / 1024.0);
        } else {
            *status.lock() = format!("Downloading: {:.1} MB", downloaded as f32 / 1024.0 / 1024.0);
        }
    }
    *download_progress.lock() = 1.0;

    *status.lock() = "Download complete. Extracting model...".to_string();
    let zip_file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;

    let mut has_toplevel = false;
    if archive.len() > 0 {
        if let Ok(entry) = archive.by_index(0) {
            if entry.name().starts_with("GAME-1.0.3-large-onnx/") {
                has_toplevel = true;
            }
        }
    }

    let extract_dest = if has_toplevel {
        models_base_dir.to_path_buf()
    } else {
        models_base_dir.join("GAME-1.0.3-large-onnx")
    };

    fs::create_dir_all(&extract_dest)?;
    let archive_len = archive.len();

    for i in 0..archive_len {
        let mut file = archive.by_index(i)?;
        let outpath = match file.enclosed_name() {
            Some(path) => extract_dest.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            io::copy(&mut file, &mut outfile)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))?;
            }
        }
        
        *extract_progress.lock() = (i + 1) as f32 / archive_len as f32;
        *status.lock() = format!("Extracting files: {} / {} ({}%)", i + 1, archive_len, ((i + 1) * 100) / archive_len);
    }
    *extract_progress.lock() = 1.0;

    *status.lock() = "Cleaning up installer files...".to_string();
    let _ = fs::remove_file(&zip_path);
    Ok(())
}

struct InstallerApp {
    status: Arc<Mutex<String>>,
    download_progress: Arc<Mutex<f32>>,
    extract_progress: Arc<Mutex<f32>>,
    is_running: Arc<Mutex<bool>>,
    is_complete: Arc<Mutex<bool>>,
    error_message: Arc<Mutex<Option<String>>>,
    model_install_path: String,
    clap_found: bool,
    vst3_found: bool,
    exe_dir: PathBuf,
}

impl InstallerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize visuals
        let mut visuals = egui::Visuals::dark();
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(139, 92, 246); // purple
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(124, 58, 237); // dark purple
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(45, 55, 72); // Slate
        cc.egui_ctx.set_visuals(visuals);

        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let clap_found = exe_dir.join("copycat.clap").exists();
        let vst3_found = exe_dir.join("copycat.vst3").exists();

        let default_parent_dir = {
            #[cfg(target_os = "windows")]
            {
                if let Ok(profile) = std::env::var("USERPROFILE") {
                    PathBuf::from(profile).join("copycat").join("models")
                } else {
                    PathBuf::from("C:\\").join("copycat").join("models")
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                if let Ok(home) = std::env::var("HOME") {
                    PathBuf::from(home).join(".local").join("share").join("copycat").join("models")
                } else {
                    PathBuf::from("/tmp").join("copycat").join("models")
                }
            }
        };

        Self {
            status: Arc::new(Mutex::new("Ready to install".to_string())),
            download_progress: Arc::new(Mutex::new(0.0)),
            extract_progress: Arc::new(Mutex::new(0.0)),
            is_running: Arc::new(Mutex::new(false)),
            is_complete: Arc::new(Mutex::new(false)),
            error_message: Arc::new(Mutex::new(None)),
            model_install_path: default_parent_dir.to_string_lossy().to_string(),
            clap_found,
            vst3_found,
            exe_dir,
        }
    }
}

impl eframe::App for InstallerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Trigger UI updates during installation
        if *self.is_running.lock() {
            ctx.request_repaint();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.heading(egui::RichText::new("Copycat Voice-to-MIDI").size(24.0).strong().color(egui::Color32::from_rgb(167, 139, 250)));
                ui.label("AI Vocal Transcription Plugin & Checkpoint Installer");
                ui.add_space(15.0);
            });

            // Group: File status
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Plugin Source Status:").strong());
                
                ui.horizontal(|ui| {
                    if self.clap_found {
                        ui.label(egui::RichText::new(" ✔ ").color(egui::Color32::GREEN));
                        ui.label("copycat.clap found");
                    } else {
                        ui.label(egui::RichText::new(" ❌ ").color(egui::Color32::LIGHT_RED));
                        ui.label("copycat.clap not found in installer directory");
                    }
                });

                ui.horizontal(|ui| {
                    if self.vst3_found {
                        ui.label(egui::RichText::new(" ✔ ").color(egui::Color32::GREEN));
                        ui.label("copycat.vst3 found");
                    } else {
                        ui.label(egui::RichText::new(" ❌ ").color(egui::Color32::LIGHT_RED));
                        ui.label("copycat.vst3 not found in installer directory");
                    }
                });
            });

            ui.add_space(10.0);

            // Group: Directory Select
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.label(egui::RichText::new("Model Destination Directory:").strong());
                
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut self.model_install_path);
                    if ui.add_enabled(!*self.is_running.lock() && !*self.is_complete.lock(), egui::Button::new("Browse...")).clicked() {
                        if let Some(folder) = rfd::FileDialog::new()
                            .set_directory(&self.model_install_path)
                            .pick_folder()
                        {
                            self.model_install_path = folder.to_string_lossy().to_string();
                        }
                    }
                });
                ui.label(egui::RichText::new(format!(
                    "Model will be downloaded to: {}/GAME-1.0.3-large-onnx",
                    self.model_install_path.trim_end_matches('/')
                )).size(11.0).color(egui::Color32::GRAY));
            });

            ui.add_space(15.0);

            // Progress / Status Box
            let status_msg = self.status.lock().clone();
            let is_running = *self.is_running.lock();
            let is_complete = *self.is_complete.lock();
            let err_opt = self.error_message.lock().clone();

            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label("Status: ");
                    ui.label(egui::RichText::new(&status_msg).strong());
                });

                if is_running {
                    let dl = *self.download_progress.lock();
                    let ext = *self.extract_progress.lock();

                    ui.add_space(5.0);
                    ui.label(format!("Downloading: {:.0}%", dl * 100.0));
                    ui.add(egui::ProgressBar::new(dl).show_percentage());

                    if dl >= 0.99 {
                        ui.add_space(5.0);
                        ui.label(format!("Extracting: {:.0}%", ext * 100.0));
                        ui.add(egui::ProgressBar::new(ext).show_percentage());
                    }
                }

                if is_complete {
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new("✔ Installation was fully successful! You can load Copycat in your DAW now.").color(egui::Color32::GREEN).strong());
                }

                if let Some(ref err) = err_opt {
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new(format!("❌ Installation Failed:\n{}", err)).color(egui::Color32::LIGHT_RED).strong());
                }
            });

            ui.add_space(20.0);

            // Button actions
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if is_complete || err_opt.is_some() {
                        if ui.button("Close").clicked() {
                            std::process::exit(0);
                        }
                    } else {
                        let btn_text = if is_running { "Installing..." } else { "Start Installation" };
                        let has_any_plugin = self.clap_found || self.vst3_found;
                        let enabled = !is_running && has_any_plugin;

                        let install_btn = ui.add_enabled(enabled, egui::Button::new(btn_text).min_size(egui::vec2(120.0, 30.0)));
                        if install_btn.clicked() {
                            let status = self.status.clone();
                            let download_progress = self.download_progress.clone();
                            let extract_progress = self.extract_progress.clone();
                            let is_running = self.is_running.clone();
                            let is_complete = self.is_complete.clone();
                            let error_message = self.error_message.clone();
                            let exe_dir = self.exe_dir.clone();
                            let model_install_path = self.model_install_path.clone();

                            *is_running.lock() = true;
                            *is_complete.lock() = false;
                            *error_message.lock() = None;

                            std::thread::spawn(move || {
                                let run = || -> anyhow::Result<()> {
                                    install_plugin_files(&exe_dir, &status)?;
                                    download_and_extract_model(&model_install_path, &status, &download_progress, &extract_progress)?;
                                    Ok(())
                                };

                                match run() {
                                    Ok(_) => {
                                        *status.lock() = "Installation complete!".to_string();
                                        *is_complete.lock() = true;
                                    }
                                    Err(e) => {
                                        *status.lock() = format!("Failed: {}", e);
                                        *error_message.lock() = Some(e.to_string());
                                    }
                                }
                                *is_running.lock() = false;
                            });
                        }
                    }
                });
            });
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([540.0, 430.0])
            .with_resizable(false)
            .with_title("Copycat Installer"),
        ..Default::default()
    };

    eframe::run_native(
        "org.openvpi.copycat.installer",
        options,
        Box::new(|cc| Ok(Box::new(InstallerApp::new(cc)))),
    )
}
