use std::path::Path;

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") || !target.contains("x86_64") {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dll_path = Path::new(&out_dir).join("onnxruntime.dll");

    if dll_path.exists() {
        copy_to_profile(&dll_path);
        return;
    }

    let url = "https://github.com/microsoft/onnxruntime/releases/download/v1.21.0/onnxruntime-win-x64-1.21.0.zip";
    let zip_path = Path::new(&out_dir).join("onnxruntime.zip");

    eprintln!("Downloading onnxruntime.dll...");

    let ok = if cfg!(target_os = "windows") {
        download_powershell(url, &zip_path) && extract_powershell(&zip_path, &out_dir)
    } else {
        download_curl(url, &zip_path) && extract_unzip(&zip_path, Path::new(&out_dir))
    };

    if ok && dll_path.exists() {
        copy_to_profile(&dll_path);
        eprintln!("onnxruntime.dll ready");
    } else {
        eprintln!("Warning: onnxruntime.dll not downloaded, transcription won't work");
    }
}

fn copy_to_profile(dll: &Path) {
    let out_dir = match std::env::var("OUT_DIR") {
        Ok(val) => val,
        Err(_) => return,
    };
    let out_path = Path::new(&out_dir);
    
    // Walk up 3 levels from OUT_DIR (out -> build/crate-id -> build -> profile)
    let profile_dir = match out_path.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
        Some(p) => p,
        None => return,
    };
    
    let dest = profile_dir.join("onnxruntime.dll");
    
    eprintln!("copy_to_profile: copying from {:?} to {:?}", dll, dest);
    if let Err(e) = std::fs::create_dir_all(dest.parent().unwrap()) {
        eprintln!("copy_to_profile: failed to create dir: {:?}", e);
    }
    match std::fs::copy(dll, &dest) {
        Ok(_) => eprintln!("copy_to_profile: successfully copied dll"),
        Err(e) => eprintln!("copy_to_profile: failed to copy dll: {:?}", e),
    }
}

fn download_curl(url: &str, dest: &Path) -> bool {
    std::process::Command::new("curl")
        .args(["-fLo", &dest.to_string_lossy(), url])
        .status().ok().map_or(false, |s| s.success())
}

fn extract_unzip(zip: &Path, out: &Path) -> bool {
    std::process::Command::new("unzip")
        .args(["-j", &zip.to_string_lossy(), "onnxruntime-win-x64-*/onnxruntime.dll", "-d", &out.to_string_lossy()])
        .status().ok().map_or(false, |s| s.success())
}

fn download_powershell(url: &str, dest: &Path) -> bool {
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
            &format!("Invoke-WebRequest -Uri '{}' -OutFile '{}'", url, dest.display())])
        .status();
    match status {
        Ok(s) if s.success() => true,
        _ => {
            download_curl(url, dest)
        }
    }
}

fn extract_powershell(zip: &Path, out: &str) -> bool {
    let status = std::process::Command::new("tar")
        .args(["-xf", &zip.to_string_lossy(), "-C", out, "--wildcards", "*/onnxruntime.dll", "--strip-components=1"])
        .status();
    match status {
        Ok(s) if s.success() => true,
        _ => {
            std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command",
                    &format!("Expand-Archive -Path '{}' -DestinationPath '{}' -Force", zip.display(), out)])
                .status().ok().map_or(false, |s| s.success())
        }
    }
}
