use dioxus::prelude::*;

/// Props per il componente SVG generico
#[derive(Props, Clone, PartialEq)]
pub struct SvgIconProps {
    /// Contenuto SVG (path, circle, line, ecc.)
    pub children: Element,
    /// Dimensione dell'icona (default: "24")
    #[props(default = "24".to_string())]
    pub size: String,
    /// Spessore del tratto (default: "2")
    #[props(default = "2".to_string())]
    pub stroke_width: String,
    /// Classe CSS aggiuntiva
    #[props(default)]
    pub class: Option<String>,
    /// Colore di riempimento per icone filled (default: "none")
    #[props(default = "none".to_string())]
    pub fill: String,
    /// Colore del tratto (default: "currentColor" per ereditare dal testo)
    #[props(default = "currentColor".to_string())]
    pub stroke: String,
}

/// Componente SVG generico riutilizzabile
///
/// Fornisce attributi SVG standardizzati con possibilità di
/// personalizzare dimensioni, spessore tratto, colore fill e stroke.
///
/// Per icone stroke-based (default): non specificare fill, usa stroke
/// Per icone filled: specifica fill, può omettere stroke
#[component]
pub fn SvgIcon(props: SvgIconProps) -> Element {
    let class_str = props.class.unwrap_or_default();

    rsx! {
        svg {
            xmlns: "http://www.w3.org/2000/svg",
            width: "{props.size}",
            height: "{props.size}",
            view_box: "0 0 24 24",
            fill: "{props.fill}",
            stroke: "{props.stroke}",
            stroke_width: "{props.stroke_width}",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            class: "{class_str}",
            {props.children}
        }
    }
}
