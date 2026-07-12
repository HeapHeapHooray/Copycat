use std::fs;
use std::io::{self, Read, Write, Seek};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::Mutex;
use eframe::egui;

const CHECKPOINT_URL: &str = "https://github.com/openvpi/GAME/releases/download/v1.0.3/GAME-1.0.3-large-onnx.zip";

// A wrapper that retries on "Overlapped I/O pending" (997) or "Sharing violation" (32)
pub struct RobustIO<T> {
    inner: T,
}

impl<T> RobustIO<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }
}

impl<T: Read> Read for RobustIO<T> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut retries = 5;
        let mut delay = std::time::Duration::from_millis(50);
        loop {
            match self.inner.read(buf) {
                Ok(n) => return Ok(n),
                Err(e) => {
                    let code = e.raw_os_error();
                    if (code == Some(997) || code == Some(32)) && retries > 0 {
                        retries -= 1;
                        std::thread::sleep(delay);
                        delay *= 2;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }
}

impl<T: Write> Write for RobustIO<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut retries = 5;
        let mut delay = std::time::Duration::from_millis(50);
        loop {
            match self.inner.write(buf) {
                Ok(n) => return Ok(n),
                Err(e) => {
                    let code = e.raw_os_error();
                    if (code == Some(997) || code == Some(32)) && retries > 0 {
                        retries -= 1;
                        std::thread::sleep(delay);
                        delay *= 2;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut retries = 5;
        let mut delay = std::time::Duration::from_millis(50);
        loop {
            match self.inner.flush() {
                Ok(()) => return Ok(()),
                Err(e) => {
                    let code = e.raw_os_error();
                    if (code == Some(997) || code == Some(32)) && retries > 0 {
                        retries -= 1;
                        std::thread::sleep(delay);
                        delay *= 2;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }
}

impl<T: Seek> Seek for RobustIO<T> {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        let mut retries = 5;
        let mut delay = std::time::Duration::from_millis(50);
        loop {
            match self.inner.seek(pos) {
                Ok(n) => return Ok(n),
                Err(e) => {
                    let code = e.raw_os_error();
                    if (code == Some(997) || code == Some(32)) && retries > 0 {
                        retries -= 1;
                        std::thread::sleep(delay);
                        delay *= 2;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }
}

// Robust filesystem helpers that retry on "Overlapped I/O pending" (997) or "Sharing violation" (32)
fn robust_create_dir_all(path: &Path) -> std::io::Result<()> {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match fs::create_dir_all(path) {
            Ok(_) => return Ok(()),
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

fn robust_copy(from: &Path, to: &Path) -> std::io::Result<u64> {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match fs::copy(from, to) {
            Ok(bytes) => return Ok(bytes),
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

fn robust_file_create(path: &Path) -> std::io::Result<fs::File> {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match fs::File::create(path) {
            Ok(file) => return Ok(file),
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

fn robust_file_open(path: &Path) -> std::io::Result<fs::File> {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match fs::File::open(path) {
            Ok(file) => return Ok(file),
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

fn robust_remove_dir_all(path: &Path) -> std::io::Result<()> {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match fs::remove_dir_all(path) {
            Ok(_) => return Ok(()),
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

fn robust_remove_file(path: &Path) -> std::io::Result<()> {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match fs::remove_file(path) {
            Ok(_) => return Ok(()),
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

fn robust_read_dir(path: &Path) -> std::io::Result<fs::ReadDir> {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match fs::read_dir(path) {
            Ok(read_dir) => return Ok(read_dir),
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

fn robust_file_type(entry: &fs::DirEntry) -> io::Result<fs::FileType> {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match entry.file_type() {
            Ok(ty) => return Ok(ty),
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                return Err(e);
            }
        }
    }
}

fn robust_exists(path: &Path) -> bool {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match path.metadata() {
            Ok(_) => return true,
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                if e.kind() == std::io::ErrorKind::NotFound {
                    return false;
                }
                return false;
            }
        }
    }
}

fn robust_is_dir(path: &Path) -> bool {
    let mut retries = 5;
    let mut delay = std::time::Duration::from_millis(50);
    loop {
        match path.metadata() {
            Ok(meta) => return meta.is_dir(),
            Err(e) => {
                let code = e.raw_os_error();
                if (code == Some(997) || code == Some(32)) && retries > 0 {
                    retries -= 1;
                    std::thread::sleep(delay);
                    delay *= 2;
                    continue;
                }
                return false;
            }
        }
    }
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    robust_create_dir_all(dst.as_ref())?;
    let mut entries = robust_read_dir(src.as_ref())?;
    loop {
        let entry = match entries.next() {
            Some(Ok(entry)) => entry,
            Some(Err(e)) => return Err(e),
            None => break,
        };
        let ty = robust_file_type(&entry)?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            robust_copy(&entry.path(), &dst.as_ref().join(entry.file_name()))?;
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
        if robust_exists(&clap_src) {
            let sys_clap_dir = PathBuf::from(&common_files).join("CLAP");
            let user_clap_dir = local_app_data.as_ref().map(|lad| PathBuf::from(lad).join("Programs").join("Common").join("CLAP"));

            *status.lock() = "Installing CLAP plugin...".to_string();
            let mut success = false;
            let mut errors = Vec::new();

            let target_sys = sys_clap_dir.join("copycat.clap");
            
            // Try system CLAP
            match robust_create_dir_all(&sys_clap_dir) {
                Ok(_) => {
                    match robust_copy(&clap_src, &target_sys) {
                        Ok(_) => {
                            if robust_exists(&dll_src) {
                                if let Err(e) = robust_copy(&dll_src, &sys_clap_dir.join("onnxruntime.dll")) {
                                    errors.push(format!("Failed to copy onnxruntime.dll to system CLAP: {}", e));
                                }
                            }
                            *status.lock() = "CLAP installed to system CLAP directory.".to_string();
                            success = true;
                        }
                        Err(e) => {
                            errors.push(format!("System CLAP copy failed: {}. Make sure your DAW is closed.", e));
                        }
                    }
                }
                Err(e) => {
                    errors.push(format!("System CLAP dir creation failed: {}", e));
                }
            }

            // Try user local path if system path failed
            if !success {
                if let Some(ref ucd) = user_clap_dir {
                    let target_user = ucd.join("copycat.clap");
                    match robust_create_dir_all(ucd) {
                        Ok(_) => {
                            match robust_copy(&clap_src, &target_user) {
                                Ok(_) => {
                                    if robust_exists(&dll_src) {
                                        if let Err(e) = robust_copy(&dll_src, &ucd.join("onnxruntime.dll")) {
                                            errors.push(format!("Failed to copy onnxruntime.dll to user CLAP: {}", e));
                                        }
                                    }
                                    *status.lock() = "CLAP installed to user CLAP directory.".to_string();
                                    success = true;
                                }
                                Err(e) => {
                                    errors.push(format!("User CLAP copy failed: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            errors.push(format!("User CLAP dir creation failed: {}", e));
                        }
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
        if robust_exists(&vst3_src) && robust_is_dir(&vst3_src) {
            let sys_vst3_dir = PathBuf::from(&common_files).join("VST3");
            let user_vst3_dir = local_app_data.as_ref().map(|lad| PathBuf::from(lad).join("Programs").join("Common").join("VST3"));

            *status.lock() = "Installing VST3 plugin...".to_string();
            let mut success = false;
            let mut errors = Vec::new();

            let target_sys = sys_vst3_dir.join("copycat.vst3");
            let _ = robust_remove_dir_all(&target_sys); // clean old
            
            match copy_dir_all(&vst3_src, &target_sys) {
                Ok(_) => {
                    *status.lock() = "VST3 installed to system VST3 directory.".to_string();
                    success = true;
                }
                Err(e) => {
                    errors.push(format!("System VST3 copy failed: {}. Make sure your DAW is closed.", e));
                }
            }

            if !success {
                if let Some(ref uvd) = user_vst3_dir {
                    let target_user = uvd.join("copycat.vst3");
                    let _ = robust_remove_dir_all(&target_user); // clean old
                    match copy_dir_all(&vst3_src, &target_user) {
                        Ok(_) => {
                            *status.lock() = "VST3 installed to user VST3 directory.".to_string();
                            success = true;
                        }
                        Err(e) => {
                            errors.push(format!("User VST3 copy failed: {}", e));
                        }
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
        if robust_exists(&clap_src) {
            let clap_dir = home_path.join(".clap");
            let target = clap_dir.join("copycat.clap");
            *status.lock() = "Installing CLAP to ~/.clap/".to_string();
            robust_create_dir_all(&clap_dir)?;
            robust_copy(&clap_src, &target)?;
            clap_installed = true;
        }

        // 2. VST3 Installation
        if robust_exists(&vst3_src) && robust_is_dir(&vst3_src) {
            let vst3_dir = home_path.join(".vst3");
            let target = vst3_dir.join("copycat.vst3");
            *status.lock() = "Installing VST3 to ~/.vst3/".to_string();
            let _ = robust_remove_dir_all(&target);
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
    use anyhow::Context;
    let models_base_dir = Path::new(model_install_path);
    robust_create_dir_all(models_base_dir)
        .context("Failed to create models directory")?;
    let zip_path = models_base_dir.join("download.zip");

    *status.lock() = "Connecting to download server...".to_string();
    let response = ureq::get(CHECKPOINT_URL)
        .call()
        .context("Failed to connect to the checkpoint download URL")?;
    
    let total_size = response
        .header("Content-Length")
        .and_then(|len| len.parse::<usize>().ok());

    let mut file = RobustIO::new(
        robust_file_create(&zip_path)
            .context("Failed to create temporary download ZIP file")?
    );
    let mut reader = RobustIO::new(response.into_reader());
    let mut buffer = [0; 65536];
    let mut downloaded = 0;

    loop {
        let bytes_read = reader.read(&mut buffer)
            .context("Failed reading from download stream")?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])
            .context("Failed writing chunk to temporary download ZIP file")?;
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
    let zip_file = robust_file_open(&zip_path)
        .context("Failed to open downloaded ZIP file for extraction")?;
    let mut archive = zip::ZipArchive::new(RobustIO::new(zip_file))
        .context("Failed to parse downloaded ZIP archive")?;

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

    robust_create_dir_all(&extract_dest)
        .context("Failed to create destination directory for extraction")?;
    let archive_len = archive.len();

    for i in 0..archive_len {
        let mut file = archive.by_index(i)
            .with_context(|| format!("Failed to read archive index {}", i))?;
        let outpath = match file.enclosed_name() {
            Some(path) => extract_dest.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            robust_create_dir_all(&outpath)
                .with_context(|| format!("Failed to create extracted directory {:?}", outpath))?;
        } else {
            if let Some(p) = outpath.parent() {
                if !robust_exists(p) {
                    robust_create_dir_all(p)
                        .with_context(|| format!("Failed to create parent directory {:?}", p))?;
                }
            }
            let mut outfile = RobustIO::new(
                robust_file_create(&outpath)
                    .with_context(|| format!("Failed to create extracted file {:?}", outpath))?
            );
            io::copy(&mut file, &mut outfile)
                .with_context(|| format!("Failed to write extracted file contents to {:?}", outpath))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode))
                    .with_context(|| format!("Failed to set permissions for {:?}", outpath))?;
            }
        }
        
        *extract_progress.lock() = (i + 1) as f32 / archive_len as f32;
        *status.lock() = format!("Extracting files: {} / {} ({}%)", i + 1, archive_len, ((i + 1) * 100) / archive_len);
    }
    *extract_progress.lock() = 1.0;

    *status.lock() = "Cleaning up installer files...".to_string();
    let _ = robust_remove_file(&zip_path);
    Ok(())
}

fn get_exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

fn get_default_model_dir() -> PathBuf {
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

        let exe_dir = get_exe_dir();
        let clap_found = robust_exists(&exe_dir.join("copycat.clap"));
        let vst3_found = robust_exists(&exe_dir.join("copycat.vst3"));
        let default_parent_dir = get_default_model_dir();

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
    let mut args = std::env::args().skip(1);
    let mut silent = false;
    let mut model_dir: Option<String> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-s" | "--silent" => {
                silent = true;
            }
            "-m" | "--model-dir" => {
                if let Some(val) = args.next() {
                    model_dir = Some(val);
                } else {
                    eprintln!("Error: --model-dir requires a directory path");
                    std::process::exit(1);
                }
            }
            "-h" | "--help" => {
                println!("Usage: copycat_installer [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -s, --silent          Run the installer silently without opening the GUI");
                println!("  -m, --model-dir <DIR> Specify the destination directory for the model (default: platform-specific path)");
                println!("  -h, --help            Print help information");
                std::process::exit(0);
            }
            unknown => {
                eprintln!("Error: Unknown option '{}'", unknown);
                eprintln!("Run with -h or --help for usage information");
                std::process::exit(1);
            }
        }
    }

    if silent {
        let exe_dir = get_exe_dir();
        let clap_found = robust_exists(&exe_dir.join("copycat.clap"));
        let vst3_found = robust_exists(&exe_dir.join("copycat.vst3"));

        if !clap_found && !vst3_found {
            eprintln!("Error: No plugin files (copycat.clap or copycat.vst3) found in installer directory.");
            std::process::exit(1);
        }

        let model_install_path = model_dir.unwrap_or_else(|| {
            get_default_model_dir().to_string_lossy().to_string()
        });

        println!("Starting Copycat installation (silent mode)...");
        println!("Installer directory: {}", exe_dir.display());
        println!("Model destination: {}/GAME-1.0.3-large-onnx", model_install_path.trim_end_matches('/'));

        let status = Arc::new(Mutex::new("Ready to install".to_string()));
        let download_progress = Arc::new(Mutex::new(0.0));
        let extract_progress = Arc::new(Mutex::new(0.0));
        let is_running = Arc::new(Mutex::new(true));
        let is_complete = Arc::new(Mutex::new(false));
        let error_message = Arc::new(Mutex::new(None));

        let status_c = status.clone();
        let download_progress_c = download_progress.clone();
        let extract_progress_c = extract_progress.clone();
        let is_running_c = is_running.clone();
        let is_complete_c = is_complete.clone();
        let error_message_c = error_message.clone();
        let exe_dir_c = exe_dir.clone();
        let model_install_path_c = model_install_path.clone();

        std::thread::spawn(move || {
            let run = || -> anyhow::Result<()> {
                install_plugin_files(&exe_dir_c, &status_c)?;
                download_and_extract_model(&model_install_path_c, &status_c, &download_progress_c, &extract_progress_c)?;
                Ok(())
            };

            match run() {
                Ok(_) => {
                    *status_c.lock() = "Installation complete!".to_string();
                    *is_complete_c.lock() = true;
                }
                Err(e) => {
                    *status_c.lock() = format!("Failed: {}", e);
                    *error_message_c.lock() = Some(e.to_string());
                }
            }
            *is_running_c.lock() = false;
        });

        let mut last_status = String::new();
        let mut last_print_time = std::time::Instant::now() - std::time::Duration::from_secs(10);

        loop {
            let status_str = status.lock().clone();
            let dl = *download_progress.lock();
            let ext = *extract_progress.lock();

            let mut should_print = status_str != last_status;

            if should_print {
                let is_progress = status_str.starts_with("Downloading:") || status_str.starts_with("Extracting files:");
                if is_progress {
                    let now = std::time::Instant::now();
                    let is_done = if status_str.starts_with("Downloading:") { dl >= 0.99 } else { ext >= 0.99 };
                    if !is_done && now.duration_since(last_print_time).as_secs_f32() < 2.0 {
                        should_print = false;
                    }
                }
            }

            if should_print {
                println!("{}", status_str);
                last_status = status_str;
                last_print_time = std::time::Instant::now();
            }

            if !*is_running.lock() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let err_opt = error_message.lock().clone();
        if let Some(err) = err_opt {
            eprintln!("Error during installation: {}", err);
            std::process::exit(1);
        } else {
            println!("Installation was fully successful!");
            std::process::exit(0);
        }
    }

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
