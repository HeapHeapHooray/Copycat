fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows") || !target.contains("x86_64") {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dll_path = std::path::Path::new(&out_dir).join("onnxruntime.dll");

    if dll_path.exists() {
        return;
    }

    eprintln!("Downloading onnxruntime.dll...");

    let url = "https://github.com/microsoft/onnxruntime/releases/download/v1.21.0/onnxruntime-win-x64-1.21.0.zip";
    let zip_path = std::path::Path::new(&out_dir).join("onnxruntime.zip");

    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-Lo", &zip_path.to_string_lossy(), url]);
    if cmd.status().ok().map_or(true, |s| !s.success()) {
        eprintln!("Failed to download onnxruntime");
        return;
    }

    let mut cmd2 = std::process::Command::new("unzip");
    cmd2.args(["-j", &zip_path.to_string_lossy(), "onnxruntime-win-x64-*/onnxruntime.dll", "-d", &out_dir]);
    if cmd2.status().ok().map_or(true, |s| !s.success()) {
        eprintln!("Failed to extract onnxruntime.dll");
        return;
    }

    if dll_path.exists() {
        let cargo_target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
        let profile = std::env::var("PROFILE").unwrap_or("debug".into());
        let target_dir = std::path::Path::new(&cargo_target).join(&profile);
        let _ = std::fs::create_dir_all(&target_dir);
        let _ = std::fs::copy(&dll_path, target_dir.join("onnxruntime.dll"));

        // Also copy to bundle paths (created by nih_plug xtask bundler)
        for bundle_dll in ["copycat.dll", "copycat.vst3"] {
            let nix_dir = std::path::Path::new(&cargo_target).join("bundled").join(bundle_dll).join("Contents").join("x86_64-win");
            let win_dir = std::path::Path::new(&cargo_target).join("bundled").join(bundle_dll).join("Contents").join("x64-win");
            for dir in &[&nix_dir, &win_dir] {
                let _ = std::fs::create_dir_all(dir);
                let _ = std::fs::copy(&dll_path, dir.join("onnxruntime.dll"));
            }
        }
        // CLAP bundle is at target/bundled/copycat.clap, DLL sits next to it
        let clap_dir = std::path::Path::new(&cargo_target).join("bundled");
        let _ = std::fs::create_dir_all(&clap_dir);
        let _ = std::fs::copy(&dll_path, clap_dir.join("onnxruntime.dll"));

        eprintln!("onnxruntime.dll bundled");
    }
}
