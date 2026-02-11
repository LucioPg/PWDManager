use dioxus::prelude::*;
use dioxus_components::{Spinner, SpinnerSize};

/// Dimensioni predefinite per l'avatar
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum AvatarSize {
    #[default]
    Medium, // avatar-md (48px)
    #[allow(dead_code)]
    Large,   // avatar-lg (96px)
    #[allow(dead_code)]
    XLarge,  // avatar-xl (128px)
    XXLarge, // avatar-xl (256px)
}

impl AvatarSize {
    pub fn as_class(&self) -> &'static str {
        match self {
            AvatarSize::Medium => "avatar-md",
            AvatarSize::Large => "avatar-lg",
            AvatarSize::XLarge => "avatar-xl",
            AvatarSize::XXLarge => "avatar-2xl",
        }
    }
}

/// Varianti di stile per il bordo dell'avatar
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum AvatarBorder {
    #[default]
    Bordered, // Bordo visibile
    #[allow(dead_code)]
    None,   // Nessun bordo
    #[allow(dead_code)]
    Circle, // Bordo circolare
}

impl AvatarBorder {
    pub fn as_class(&self) -> &'static str {
        match self {
            AvatarBorder::Bordered => "avatar-bordered",
            AvatarBorder::None => "",
            AvatarBorder::Circle => "avatar-circle",
        }
    }
}

/// Componente AvatarSelector - Selettore di avatar con preview
///
/// Questo componente permette di selezionare un'immagine avatar con preview in tempo reale.
/// Include un pulsante per aprire il file picker e mostra l'immagine selezionata o un default.
///
/// # Esempio
/// ```rust
/// AvatarSelector {
///     avatar_src: get_user_avatar_with_default(selected_image.read().clone()),
///     on_pick: pick_image,
///     button_text: "Select Avatar".to_string(),
///     size: AvatarSize::Large,
///     ..Default::default()
/// }
/// ```
#[component]
pub fn AvatarSelector(
    /// URL dell'immagine avatar corrente (Data URL o percorso)
    avatar_src: String,
    /// Callback quando si clicca sul pulsante di selezione
    on_pick: EventHandler<MouseEvent>,
    /// Testo del pulsante di selezione
    #[props(default)]
    button_text: String,
    /// Dimensione dell'avatar
    #[props(default)]
    size: AvatarSize,
    /// Stile del bordo
    #[props(default)]
    border: AvatarBorder,
    /// Classe CSS aggiuntiva per il container
    #[props(default)]
    class: Option<String>,
    /// Mostra o nasconde l'ombra
    #[props(default)]
    shadow: bool,
    /// Mostra o nasconde il bordo
    #[props(default)]
    show_border: bool,
    #[props(default)] loading: Signal<bool>,
    /// Signal che indica se il dialog di selezione è aperto
    #[props(default)] is_picking: Signal<bool>,
) -> Element {
    let size_class = size.as_class();
    let border_class = if show_border { border.as_class() } else { "" };
    let shadow_class = if shadow { "shadow-lg" } else { "" };

    // Costruisci le classi CSS dinamicamente
    let img_classes = format!("avatar {} {} {}", size_class, border_class, shadow_class)
        .trim()
        .to_string();

    let container_classes = if let Some(custom_class) = class {
        format!("flex flex-col items-center gap-3 mb-4 {}", custom_class)
    } else {
        "flex flex-col items-center gap-3 mb-4".to_string()
    };
    rsx! {
    div { class: "{container_classes}",
        if loading() {
            div {class: "{img_classes} flex items-center justify-center",
                Spinner {size: SpinnerSize::Small, color: "text-success"}
                }
            }
            else {
                img {
                    class: "{img_classes}",
                    src: "{avatar_src}",
                    alt: "User Avatar"
                }
                }
                button {
                class: "btn-ghost btn-sm",
                r#type: "button",
                onclick: on_pick,
                // Disabilita il bottone se sta caricando O se il dialog è aperto
                disabled: loading() || is_picking(),
                "{button_text}"
                }
            }
    }
}
