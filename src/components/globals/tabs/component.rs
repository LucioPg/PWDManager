use dioxus::prelude::*;
use dioxus_primitives::tabs::{self, TabContentProps, TabListProps, TabTriggerProps};

/// The props for the [`Tabs`] component.
#[derive(Props, Clone, PartialEq)]
pub struct TabsProps {
    /// The class of the tabs component.
    #[props(default)]
    pub class: String,

    /// The controlled value of the active tab.
    pub value: ReadSignal<Option<String>>,

    /// The default active tab value when uncontrolled.
    #[props(default)]
    pub default_value: String,

    /// Callback fired when the active tab changes.
    #[props(default)]
    pub on_value_change: Callback<String>,

    /// Whether the tabs are disabled.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether the tabs are horizontal.
    #[props(default)]
    pub horizontal: ReadSignal<bool>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub roving_loop: ReadSignal<bool>,

    /// Additional attributes to apply to the tabs element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the tabs component.
    pub children: Element,
}

#[component]
pub fn Tabs(props: TabsProps) -> Element {
    rsx! {
        tabs::Tabs {
            class: props.class + " pwd-tabs" + " futuristic",
            value: props.value,
            default_value: props.default_value,
            on_value_change: props.on_value_change,
            disabled: props.disabled,
            horizontal: props.horizontal,
            roving_loop: props.roving_loop,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TabList(props: TabListProps) -> Element {
    rsx! {
        tabs::TabList { class: "pwd-tabs-list futuristic", attributes: props.attributes, {props.children} }
    }
}

#[component]
pub fn TabTrigger(props: TabTriggerProps) -> Element {
    rsx! {
        tabs::TabTrigger {
            class: "pwd-tabs-trigger",
            id: props.id,
            value: props.value,
            index: props.index,
            disabled: props.disabled,
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[component]
pub fn TabContent(props: TabContentProps) -> Element {
    rsx! {
        tabs::TabContent {
            class: props.class.unwrap_or_default() + " pwd-tabs-content" + " futuristic",
            value: props.value,
            id: props.id,
            index: props.index,
            attributes: props.attributes,
            {props.children}
        }
    }
}
