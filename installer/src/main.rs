use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

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

fn install_plugin_files(src_dir: &Path) -> anyhow::Result<()> {
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

            println!("Installing CLAP plugin...");
            // Try system path first
            let mut success = false;
            let mut errors = Vec::new();

            let target_sys = sys_clap_dir.join("copycat.clap");
            if fs::create_dir_all(&sys_clap_dir).is_ok() && fs::copy(&clap_src, &target_sys).is_ok() {
                if dll_src.exists() {
                    let _ = fs::copy(&dll_src, sys_clap_dir.join("onnxruntime.dll"));
                }
                println!("Successfully installed CLAP to system directory: {:?}", target_sys);
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
                        println!("Successfully installed CLAP to user directory: {:?}", target_user);
                        success = true;
                    } else {
                        errors.push("User directory write failed");
                    }
                }
            }

            if success {
                clap_installed = true;
            } else {
                println!("Warning: Failed to install CLAP. Errors: {:?}", errors);
                println!("Please run the installer as Administrator or manually copy 'copycat.clap' and 'onnxruntime.dll' to your CLAP folder.");
            }
        } else {
            println!("CLAP plugin source (copycat.clap) not found in the installer directory. Skipping CLAP.");
        }

        // 2. VST3 Installation
        if vst3_src.exists() && vst3_src.is_dir() {
            let sys_vst3_dir = PathBuf::from(&common_files).join("VST3");
            let user_vst3_dir = local_app_data.as_ref().map(|lad| PathBuf::from(lad).join("Programs").join("Common").join("VST3"));

            println!("Installing VST3 plugin...");
            let mut success = false;
            let mut errors = Vec::new();

            let target_sys = sys_vst3_dir.join("copycat.vst3");
            let _ = fs::remove_dir_all(&target_sys); // clean old
            if copy_dir_all(&vst3_src, &target_sys).is_ok() {
                println!("Successfully installed VST3 to system directory: {:?}", target_sys);
                success = true;
            } else {
                errors.push("System directory write failed (Admin permissions may be needed)");
            }

            if !success {
                if let Some(ref uvd) = user_vst3_dir {
                    let target_user = uvd.join("copycat.vst3");
                    let _ = fs::remove_dir_all(&target_user); // clean old
                    if copy_dir_all(&vst3_src, &target_user).is_ok() {
                        println!("Successfully installed VST3 to user directory: {:?}", target_user);
                        success = true;
                    } else {
                        errors.push("User directory write failed");
                    }
                }
            }

            if success {
                vst3_installed = true;
            } else {
                println!("Warning: Failed to install VST3. Errors: {:?}", errors);
                println!("Please run the installer as Administrator or manually copy 'copycat.vst3' to your VST3 folder.");
            }
        } else {
            println!("VST3 plugin source (copycat.vst3) not found in the installer directory. Skipping VST3.");
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
            println!("Installing CLAP to {:?}", target);
            fs::create_dir_all(&clap_dir)?;
            fs::copy(&clap_src, &target)?;
            println!("Successfully installed CLAP.");
            clap_installed = true;
        } else {
            println!("CLAP plugin source (copycat.clap) not found in the installer directory. Skipping CLAP.");
        }

        // 2. VST3 Installation
        if vst3_src.exists() && vst3_src.is_dir() {
            let vst3_dir = home_path.join(".vst3");
            let target = vst3_dir.join("copycat.vst3");
            println!("Installing VST3 to {:?}", target);
            let _ = fs::remove_dir_all(&target);
            copy_dir_all(&vst3_src, &target)?;
            println!("Successfully installed VST3.");
            vst3_installed = true;
        } else {
            println!("VST3 plugin source (copycat.vst3) not found in the installer directory. Skipping VST3.");
        }
    }

    if !clap_installed && !vst3_installed {
        println!("Neither copycat.clap nor copycat.vst3 was successfully installed.");
    }

    Ok(())
}

fn download_and_extract_model() -> anyhow::Result<()> {
    // Determine the target model path
    let models_base_dir = {
        #[cfg(target_os = "windows")]
        {
            if let Ok(profile) = std::env::var("USERPROFILE") {
                PathBuf::from(profile).join("copycat").join("models")
            } else {
                anyhow::bail!("USERPROFILE environment variable not found. Cannot determine model directory.");
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".local").join("share").join("copycat").join("models")
            } else {
                anyhow::bail!("HOME environment variable not found. Cannot determine model directory.");
            }
        }
    };

    fs::create_dir_all(&models_base_dir)?;
    let zip_path = models_base_dir.join("GAME-1.0.3-large-onnx.zip");

    println!("Target model directory: {:?}", models_base_dir.join("GAME-1.0.3-large-onnx"));

    // Download the zip file
    println!("Connecting to GitHub to download model checkpoint...");
    let response = ureq::get(CHECKPOINT_URL).call()?;
    
    let total_size = response
        .header("Content-Length")
        .and_then(|len| len.parse::<usize>().ok());

    println!("Downloading from: {}", CHECKPOINT_URL);
    if let Some(size) = total_size {
        println!("Total file size: {:.2} MB", size as f64 / 1024.0 / 1024.0);
    } else {
        println!("Total file size: Unknown");
    }

    let mut file = fs::File::create(&zip_path)?;
    let mut reader = response.into_reader();
    let mut buffer = [0; 65536];
    let mut downloaded = 0;
    let mut last_percent = 0;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read;

        if let Some(total) = total_size {
            let percent = (downloaded * 100) / total;
            if percent != last_percent {
                print!("\rDownloading: {}% ({:.1} MB / {:.1} MB)", percent, downloaded as f64 / 1024.0 / 1024.0, total as f64 / 1024.0 / 1024.0);
                let _ = io::stdout().flush();
                last_percent = percent;
            }
        } else {
            print!("\rDownloading: {:.1} MB", downloaded as f64 / 1024.0 / 1024.0);
            let _ = io::stdout().flush();
        }
    }
    println!("\nDownload completed. Extracting...");

    // Open and extract zip
    let zip_file = fs::File::open(&zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;

    // Check if the zip already contains a top-level directory "GAME-1.0.3-large-onnx"
    let mut has_toplevel = false;
    if archive.len() > 0 {
        if let Ok(entry) = archive.by_index(0) {
            if entry.name().starts_with("GAME-1.0.3-large-onnx/") {
                has_toplevel = true;
            }
        }
    }

    let extract_dest = if has_toplevel {
        models_base_dir.clone()
    } else {
        models_base_dir.join("GAME-1.0.3-large-onnx")
    };

    fs::create_dir_all(&extract_dest)?;

    for i in 0..archive.len() {
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
    }

    println!("Extraction completed successfully.");
    
    // Clean up zip
    let _ = fs::remove_file(&zip_path);
    Ok(())
}

fn main() {
    println!("====================================================");
    println!("            Copycat Voice-to-MIDI Installer          ");
    println!("====================================================");

    // Locate the folder of the running installer
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    println!("Installer folder: {:?}", exe_dir);

    // 1. Install CLAP/VST3 plugin files
    match install_plugin_files(&exe_dir) {
        Ok(_) => println!("Plugin installation phase finished."),
        Err(e) => println!("Error during plugin installation phase: {}", e),
    }

    println!("\n----------------------------------------------------");

    // 2. Download and install ONNX model
    match download_and_extract_model() {
        Ok(_) => println!("Model download and extraction phase finished."),
        Err(e) => println!("Error during model installation phase: {}", e),
    }

    println!("====================================================");
    println!("Installation process finished! You can now open your DAW.");
    println!("====================================================");
}
