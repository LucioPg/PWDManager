## Move to thread example
```rust



pub async fn scale_avatar(path: String) -> Result<String, Error> { // <-- Use a custom error type here
    tokio::task::spawn_blocking(move || {
        let img = image::open(&path).map_err(|e| e.to_string())?; // <-- Use a custom error type here
        let scaled = img.thumbnail(128, 128);

        // --- IL BUFFER: Qui creiamo i byte del file PNG ---
        let mut buffer = Vec::new();
        scaled.write_to(&mut std::io::Cursor::new(&mut buffer), image::ImageFormat::Png)
            .map_err(|e| e.to_string())?; // <-- Use a custom error type here

        // 2. La stringa per la UI
        let b64_string = base64::engine::general_purpose::STANDARD.encode(&buffer);
        let base64 = format!("data:image/png;base64,{}", b64_string);
    })
    .await
    .map_err(|e| e.to_string())? // <-- Use a custom error type here
}

```

In the diaoxus component:
```rust

use dioxus::prelude::*;

// Immagine di default (base64 reale o segnaposto)
const DEFAULT_B64: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

#[component]
pub fn App() -> Element {
    let mut selected_path = use_signal(|| None::<String>);
    
    // La risorsa reagisce a selected_path
    let image_resource = use_resource(move || async move {
        match selected_path() {
            Some(path) => scale_image(path).await.ok(),
            None => None,
        }
    });

    // Determiniamo cosa mostrare
    let display_src = match &*image_resource.read_unchecked() {
        Some(scaled_b64) => scaled_b64.clone(), // Immagine processata
        None if image_resource.loading() => "LOADING".to_string(), // Stato speciale per spinner
        _ => DEFAULT_B64.to_string(), // Default iniziale o errore
    };

    rsx! {
        div { class: "container",
            input {
                type: "file",
                onchange: move |evt| {
                    if let Some(file) = evt.files().and_then(|f| f.files().first()) {
                        selected_path.set(Some(file.clone()));
                    }
                }
            }

            div { class: "preview-slot",
                if display_src == "LOADING" {
                    // Sostituisci con il tuo componente Spinner
                    div { class: "spinner", "Ridimensionamento in corso..." }
                } else {
                    img { 
                        src: "{display_src}", 
                        style: "width: 128px; height: 128px; object-fit: contain; border: 1px solid #ccc;"
                    }
                }
            }
            
            if selected_path().is_some() {
                button { onclick: move |_| selected_path.set(None), "Ripristina Default" }
            }
        }
    }
}

```