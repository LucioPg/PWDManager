use std::fs;
use std::path::Path;
use std::process::Command;

/// Resolve pwd-dioxus crate path via cargo metadata (minified JSON output).
fn find_pwd_dioxus_path() -> Option<std::path::PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let bytes = stdout.as_bytes();
    let name_marker = "\"name\":\"pwd-dioxus\"";
    let mp_prefix = "\"manifest_path\":\"";

    // cargo metadata is minified (single line). "name":"pwd-dioxus" appears both
    // in dependency entries (no manifest_path) and in the packages array entry.
    // Try each occurrence and return the one that has a manifest_path.
    let mut search_from = 0;
    while search_from < bytes.len() {
        let name_pos = match stdout[search_from..].find(name_marker) {
            Some(pos) => search_from + pos,
            None => break,
        };

        // Find end of enclosing object (the '{' before name_pos is uncounted)
        let mut depth = 0i32;
        let mut obj_end = bytes.len();
        for (i, &ch) in bytes[name_pos..].iter().enumerate() {
            match ch {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == -1 {
                        obj_end = name_pos + i;
                        break;
                    }
                }
                _ => {}
            }
        }

        // Check if this object contains manifest_path
        let obj = &stdout[name_pos..obj_end];
        if let Some(mp_start) = obj.find(mp_prefix) {
            let path_start = mp_start + mp_prefix.len();
            if let Some(path_end) = obj[path_start..].find('"') {
                let manifest_path = &obj[path_start..path_start + path_end];
                let pkg_dir = Path::new(manifest_path).parent()?;
                return Some(pkg_dir.to_path_buf());
            }
        }

        search_from = obj_end;
    }

    None
}

fn windows_executable_icon() {
    let result = embed_resource::compile("build/icon.rc", embed_resource::NONE);
    result.manifest_optional().unwrap();
}

fn main() {
    println!("cargo:rerun-if-changed=assets/input.css");
    println!("cargo:rerun-if-changed=assets/input_main.css");
    println!("cargo:rerun-if-changed=src/**/*.rs");

    // Copy pwd-dioxus assets
    let assets_dir = Path::new("assets");

    if let Some(pwd_dioxus_dir) = find_pwd_dioxus_path() {
        let src_components = pwd_dioxus_dir.join("assets/components.css");
        let src_safelist = pwd_dioxus_dir.join("assets/tailwind-safelist.txt");

        if src_components.exists() {
            let dst = assets_dir.join("pwd-dioxus-components.css");
            let _ = fs::copy(&src_components, &dst);
            println!("cargo:warning=Copied components.css from pwd-dioxus");
        } else {
            println!("cargo:warning=pwd-dioxus components.css not found at {:?}", src_components);
        }

        if src_safelist.exists() {
            let dst = assets_dir.join("pwd-dioxus-safelist.txt");
            let _ = fs::copy(&src_safelist, &dst);
            println!("cargo:warning=Copied tailwind-safelist.txt from pwd-dioxus");
        } else {
            println!("cargo:warning=pwd-dioxus tailwind-safelist.txt not found at {:?}", src_safelist);
        }
    } else {
        println!("cargo:warning=pwd-dioxus not found in cargo metadata. CSS classes from pwd-dioxus may be missing.");
    }

    // Run npm build:css
    let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };

    let output = Command::new(npm_cmd)
        .args(["run", "build:css"])
        .output()
        .expect("Failed to execute npm run build:css - make sure npm is installed!");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!(
            "Tailwind CSS compilation failed:\nstdout: {}\nstderr: {}",
            stdout, stderr
        );
    }

    println!("Tailwind CSS compiled successfully!");

    #[cfg(windows)]
    windows_executable_icon();
}
