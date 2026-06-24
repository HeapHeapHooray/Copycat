use std::path::Path;

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") || !target.contains("x86_64") {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dll_path = Path::new(&out_dir).join("onnxruntime.dll");

    if dll_path.exists() {
        return;
    }

    let url = "https://github.com/microsoft/onnxruntime/releases/download/v1.21.0/onnxruntime-win-x64-1.21.0.zip";
    let zip_path = Path::new(&out_dir).join("onnxruntime.zip");

    let ok = if cfg!(target_os = "windows") {
        download_powershell(url, &zip_path) && extract_powershell(&zip_path, &out_dir)
    } else {
        download_curl(url, &zip_path) && extract_unzip(&zip_path, Path::new(&out_dir))
    };

    if ok && dll_path.exists() {
        let cargo_target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
        let profile = std::env::var("PROFILE").unwrap_or("debug".into());
        let dest = Path::new(&cargo_target).join(&profile).join("onnxruntime.dll");
        let _ = std::fs::create_dir_all(dest.parent().unwrap());
        let _ = std::fs::copy(&dll_path, &dest);
        eprintln!("onnxruntime.dll ready");
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
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
            &format!("Invoke-WebRequest -Uri '{}' -OutFile '{}'", url, dest.display())])
        .status().ok().map_or(false, |s| s.success())
}

fn extract_powershell(zip: &Path, out: &str) -> bool {
    std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
            &format!("Expand-Archive -Path '{}' -DestinationPath '{}' -Force", zip.display(), out)])
        .status().ok().map_or(false, |s| s.success())
}
