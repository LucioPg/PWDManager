use dioxus::prelude::*;

/// Varianti di stile per i pulsanti
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
}

/// Dimensioni dei pulsanti
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ButtonSize {
    #[default]
    Normal,
    Small,
    Large,
}

impl ButtonVariant {
    pub fn as_class(&self) -> &'static str {
        match self {
            ButtonVariant::Primary => "btn-primary",
            ButtonVariant::Secondary => "btn-secondary",
            ButtonVariant::Ghost => "btn-ghost",
        }
    }
}

impl ButtonSize {
    pub fn as_suffix(&self) -> &'static str {
        match self {
            ButtonSize::Normal => "",
            ButtonSize::Small => "-sm",
            ButtonSize::Large => "-lg",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ButtonType {
    #[default]
    Button,
    Submit,
}

impl ButtonType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ButtonType::Button => "button",
            ButtonType::Submit => "submit",
        }
    }
}

/// Componente per un singolo pulsante azione
#[component]
pub fn ActionButton(
    text: String,
    #[props(default)] variant: ButtonVariant,
    #[props(default)] size: ButtonSize,
    #[props(default)] block: bool,
    #[props(default)] button_type: ButtonType,
    #[props(default)] disabled: Signal<bool>,
    on_click: EventHandler<MouseEvent>,
) -> Element {
    let size_suffix = size.as_suffix();
    let variant_class = variant.as_class();

    let base_classes = if size == ButtonSize::Normal {
        variant_class.to_string()
    } else {
        format!("{}{}", variant_class, size_suffix)
    };

    let classes = if block {
        format!("{} {}", base_classes, "btn-block")
    } else {
        base_classes
    };
    // let is_disabled: bool = (*disabled.read()).into();
    rsx! {

        button {
            class: "{classes}",
            r#type: "{button_type.as_str()}",
            onclick: on_click,
            disabled: if *disabled.read() { true } else { false },
            "{text}"
        }
    }
}

/// Componente per coppia di pulsanti (es. Login + Register, Submit + Cancel)
#[component]
pub fn ActionButtons(
    primary_text: String,
    secondary_text: String,
    primary_on_click: EventHandler<MouseEvent>,
    secondary_on_click: EventHandler<MouseEvent>,
    #[props(default)]
    variant: ActionButtonsVariant,
) -> Element {
    let (primary_variant, secondary_variant, size, block) = match variant {
        ActionButtonsVariant::Auth => (
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonSize::Normal,
            true,
        ),
        ActionButtonsVariant::Nav => (
            ButtonVariant::Primary,
            ButtonVariant::Secondary,
            ButtonSize::Small,
            false,
        ),
        ActionButtonsVariant::Ghost => (
            ButtonVariant::Ghost,
            ButtonVariant::Ghost,
            ButtonSize::Normal,
            false,
        ),
    };

    rsx! {
        ActionButton {
            text: primary_text,
            variant: primary_variant,
            size,
            block,
            button_type: ButtonType::Submit,
            on_click: primary_on_click,
        }
        ActionButton {
            text: secondary_text,
            variant: secondary_variant,
            size,
            block,
            button_type: ButtonType::Button,
            on_click: secondary_on_click,
        }
    }
}

/// Preimpostazioni per le combinazioni di pulsanti più comuni
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ActionButtonsVariant {
    #[default]
    Auth, // Pulsanti di autenticazione (login/register)
    Nav,   // Pulsanti di navigazione (navbar)
    Ghost, // Pulsanti ghost
}
