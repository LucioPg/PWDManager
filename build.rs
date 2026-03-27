use std::fs;
use std::path::Path;
use std::process::Command;

const PWD_DIOXUS_VERSION: &str = "0.2.4";
const PWD_DIOXUS_GITHUB_RAW: &str = "https://raw.githubusercontent.com/LucioPg/pwd-dioxus";

/// Write content to file only if it differs from existing content.
/// This prevents unnecessary file modifications that trigger hotreload loops.
fn write_if_changed(path: &Path, content: &str) -> std::io::Result<bool> {
    // Check if file exists and has same content
    if path.exists()
        && let Ok(existing) = fs::read_to_string(path)
        && existing == content
    {
        return Ok(false); // No change needed
    }

    // Write new content
    fs::write(path, content)?;
    Ok(true) // File was written
}

/// Copy file only if content differs from destination.
/// This prevents unnecessary file modifications that trigger hotreload loops.
fn copy_if_changed(src: &Path, dst: &Path) -> std::io::Result<bool> {
    if !src.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Source file not found: {:?}", src),
        ));
    }

    // Check if destination exists and has same content
    if dst.exists() {
        let src_content = fs::read(src)?;
        let dst_content = fs::read(dst)?;
        if src_content == dst_content {
            return Ok(false); // No change needed
        }
    }

    // Copy file
    fs::copy(src, dst)?;
    Ok(true) // File was copied
}

#[cfg(windows)]
fn windows_executable_icon() {
    println!("executable icon for windows os");
    #[cfg(windows)]
    {
        winres::WindowsResource::new()
            .set_icon("icons/icon.ico")
            .compile()
            .unwrap()
    }
}

/// Download a file from GitHub raw URL (fallback when local cache is not available)
fn download_from_github(
    filename: &str,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!(
        "{}/v{}/{}",
        PWD_DIOXUS_GITHUB_RAW, PWD_DIOXUS_VERSION, filename
    );

    println!("cargo:warning=Downloading {} from GitHub...", url);

    // Try curl first, fall back to PowerShell on Windows
    let output = Command::new("curl")
        .args(["-sL", "-o", output_path.to_str().unwrap(), &url])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            println!("cargo:warning=Successfully downloaded {}", filename);
            Ok(())
        }
        _ => {
            // Fallback: try with PowerShell on Windows
            if cfg!(windows) {
                let ps_output = Command::new("powershell")
                    .args([
                        "-Command",
                        &format!(
                            "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                            url,
                            output_path.to_str().unwrap()
                        ),
                    ])
                    .output();

                match ps_output {
                    Ok(o) if o.status.success() => {
                        println!(
                            "cargo:warning=Successfully downloaded {} via PowerShell",
                            filename
                        );
                        Ok(())
                    }
                    _ => Err(format!("Failed to download {} from GitHub", filename).into()),
                }
            } else {
                Err(format!("Failed to download {} from GitHub", filename).into())
            }
        }
    }
}

/// Find pwd-dioxus in Cargo's git cache
/// Looks for the version that contains COMPONENT_CSS constant
fn find_pwd_dioxus_in_cargo_cache() -> Option<std::path::PathBuf> {
    // Cargo stores git dependencies in ~/.cargo/git/checkouts/
    let home = std::env::var("CARGO_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| {
            if cfg!(windows) {
                std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string())
            } else {
                "/".to_string()
            }
        });
        format!("{}/.cargo", home)
    });

    let checkouts_dir = Path::new(&home).join("git").join("checkouts");

    if !checkouts_dir.exists() {
        return None;
    }

    // Look for pwd-dioxus-* directory
    for entry in fs::read_dir(&checkouts_dir).ok()?.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("pwd-dioxus-") {
            // Inside there should be hash directories
            for hash_entry in fs::read_dir(entry.path()).ok()?.flatten() {
                let lib_path = hash_entry.path().join("src").join("lib.rs");
                let css_path = hash_entry.path().join("assets").join("components.css");

                if lib_path.exists() && css_path.exists() {
                    // Check if this version has COMPONENT_CSS constant
                    if let Ok(lib_content) = fs::read_to_string(&lib_path)
                        && lib_content.contains("COMPONENT_CSS")
                    {
                        return Some(hash_entry.path());
                    }
                }
            }
        }
    }

    None
}

/// Extract Tailwind classes from lib.rs content by parsing TAILWIND_CLASSES_USED
fn extract_tailwind_classes(lib_rs_content: &str) -> Vec<String> {
    let mut classes = Vec::new();
    let mut in_array = false;

    for line in lib_rs_content.lines() {
        let trimmed = line.trim();

        // Detect start of TAILWIND_CLASSES_USED array
        if trimmed.contains("TAILWIND_CLASSES_USED") && trimmed.contains('&') {
            in_array = true;
            continue;
        }

        if in_array {
            // End of array
            if trimmed.starts_with(']') {
                break;
            }

            // Skip comments
            if trimmed.starts_with("//") {
                continue;
            }

            // Extract string literal
            if let Some(start) = trimmed.find('"')
                && let Some(end) = trimmed[start + 1..].find('"')
            {
                let class = &trimmed[start + 1..start + 1 + end];
                // Skip section headers (=== ...)
                if !class.starts_with("===") && !class.is_empty() {
                    classes.push(class.to_string());
                }
            }
        }
    }

    classes
}

/// Generate a dummy Rust file containing Tailwind classes for scanning
fn generate_tailwind_classes_file(
    classes: &[String],
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut content = String::new();
    content.push_str("// Auto-generated by build.rs - DO NOT EDIT\n");
    content.push_str("// This file contains Tailwind classes used by pwd-dioxus\n");
    content.push_str("// Tailwind will scan this file to generate the necessary CSS\n\n");
    content.push_str("fn _pwd_dioxus_classes() {\n");

    for class in classes {
        content.push_str(&format!("    // class: \"{}\"\n", class));
        content.push_str(&format!("    let _ = \"{}\";\n", class));
    }

    content.push_str("}\n");

    let changed = write_if_changed(output_path, &content)?;
    if changed {
        println!(
            "cargo:warning=Generated {} Tailwind classes from pwd-dioxus",
            classes.len()
        );
    }
    Ok(())
}

/// Extract pwd-dioxus assets for Tailwind build
fn extract_pwd_dioxus_assets() -> Result<(), Box<dyn std::error::Error>> {
    let assets_dir = Path::new("assets");
    let components_css_path = assets_dir.join("pwd-dioxus-components.css");
    let classes_rs_path = assets_dir.join("pwd-dioxus-classes.rs");

    // Try to find pwd-dioxus in Cargo's cache first
    if let Some(cache_path) = find_pwd_dioxus_in_cargo_cache() {
        // Copy components.css from cache (only if changed)
        let cached_css = cache_path.join("assets").join("components.css");
        if cached_css.exists() {
            let changed = copy_if_changed(&cached_css, &components_css_path)?;
            if changed {
                println!("cargo:warning=Updated components.css from Cargo cache");
            }
        }

        // Extract classes from lib.rs in cache
        let cached_lib_rs = cache_path.join("src").join("lib.rs");
        if cached_lib_rs.exists() {
            let lib_content = fs::read_to_string(&cached_lib_rs)?;
            let classes = extract_tailwind_classes(&lib_content);
            generate_tailwind_classes_file(&classes, &classes_rs_path)?;
        }
    } else {
        // Fallback: download from GitHub
        println!("cargo:warning=pwd-dioxus not found in Cargo cache, downloading from GitHub...");

        // Download components.css
        download_from_github("assets/components.css", &components_css_path)?;

        // Download lib.rs to extract TAILWIND_CLASSES_USED
        let temp_lib_rs = assets_dir.join(".pwd-dioxus-lib-rs.tmp");
        download_from_github("src/lib.rs", &temp_lib_rs)?;

        let lib_content = fs::read_to_string(&temp_lib_rs)?;
        let classes = extract_tailwind_classes(&lib_content);
        generate_tailwind_classes_file(&classes, &classes_rs_path)?;

        // Clean up temp file
        let _ = fs::remove_file(&temp_lib_rs);
    }

    // Note: NOT adding rerun-if-changed for generated files to avoid rebuild loops
    // The files will be regenerated if build.rs runs for other reasons

    Ok(())
}

fn main() {
    // Compila Tailwind CSS solo se i file sorgente sono cambiati
    println!("cargo:rerun-if-changed=assets/input.css");
    println!("cargo:rerun-if-changed=assets/input_main.css");
    println!("cargo:rerun-if-changed=src/**/*.rs");

    // Extract pwd-dioxus CSS and classes before Tailwind build
    if let Err(e) = extract_pwd_dioxus_assets() {
        println!("cargo:warning=Failed to extract pwd-dioxus assets: {}", e);
        println!("cargo:warning=Tailwind classes from pwd-dioxus may not be generated.");
        println!("cargo:warning=Make sure you have internet access or run 'cargo fetch' first.");
    }

    // Esegue npm run build:css
    // Su Windows usiamo npm.cmd, su Unix usiamo npm
    let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };

    let output = Command::new(npm_cmd)
        .args(["run", "build:css"])
        .output()
        .expect("Failed to execute npm run build:css - assicurati di avere npm installato!");

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
