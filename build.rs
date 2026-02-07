use std::process::Command;
#[cfg(windows)]
fn windows_executable_icon(){
    println!("executable icon for windows os");
    #[cfg(windows)]
    {
        winres::WindowsResource::new()
            .set_icon("icons/icon.ico")
            .compile().unwrap()
    }
}


fn main() {
    // Compila Tailwind CSS solo se i file sorgente sono cambiati
    println!("cargo:rerun-if-changed=assets/input.css");
    println!("cargo:rerun-if-changed=assets/input_main.css");
    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=src/**/*.rs");

    // Esegue npm run build:css
    // Su Windows usiamo npm.cmd, su Unix usiamo npm
    let npm_cmd = if cfg!(windows) { "npm.cmd" } else { "npm" };

    let output = Command::new(npm_cmd)
        .args(&["run", "build:css"])
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
