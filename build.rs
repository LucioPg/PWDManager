use std::fs;
use std::path::Path;
use std::process::Command;

/// Resolve pwd-dioxus crate path via cargo metadata.
fn find_pwd_dioxus_path() -> Option<std::path::PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1", "--no-deps"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find the pwd-dioxus package and get its manifest_path
    // We need to find "pwd-dioxus" in the packages array
    let mut in_packages = false;
    let mut brace_depth = 0;
    let mut in_pwd_dioxus = false;
    let mut found = false;
    let mut manifest_path = String::new();

    for line in stdout.lines() {
        let trimmed = line.trim();

        if !in_packages {
            if trimmed == "\"packages\": [" || trimmed == "\"packages\":[" {
                in_packages = true;
                brace_depth = 0;
            }
            continue;
        }

        // Track nested braces
        for ch in trimmed.chars() {
            match ch {
                '{' => brace_depth += 1,
                '}' => {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        in_pwd_dioxus = false;
                        if found {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        if found {
            if trimmed.contains("\"manifest_path\"") {
                // Extract path from: "manifest_path": "/path/to/Cargo.toml"
                if let Some(start) = trimmed.find('"').and_then(|i| {
                    trimmed[i + 1..].find('"').map(|j| Some(i + 1 + j))
                }).flatten() {
                    if let Some(end) = trimmed[start..].find('"') {
                        manifest_path = trimmed[start..start + end].to_string();
                        found = true;
                    }
                }
            }
        }

        if trimmed.contains("\"name\": \"pwd-dioxus\"") || trimmed.contains("\"name\":\"pwd-dioxus\"") {
            in_pwd_dioxus = true;
        }
    }

    if manifest_path.is_empty() {
        return None;
    }

    // manifest_path points to Cargo.toml, we need the parent directory
    let pkg_dir = Path::new(&manifest_path).parent()?;
    Some(pkg_dir.to_path_buf())
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
