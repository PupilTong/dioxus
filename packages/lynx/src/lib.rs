pub mod elements;
pub mod events;
mod renderer;

pub use dioxus_lynx_macro::{CSS, inline_image};
pub use dioxus_lynx_sys as lynx_sys;
pub use dioxus_lynx_sys::css;
pub use events::{LynxEvent, LynxEventData};
pub use renderer::{LynxApp, LynxMutations, mount, with_app};

pub fn launch(root: fn() -> dioxus_core::Element) {
    renderer::install_app(renderer::mount(root));
}

pub fn launch_with_stylesheet(root: fn() -> dioxus_core::Element, stylesheet: css::CSSTokenStream) {
    lynx_sys::raw::replace_style_sheets_tokens(stylesheet);
    launch(root);
}

#[cfg(target_arch = "wasm32")]
#[unsafe(no_mangle)]
extern "C" fn __dioxus_lynx_event_dispatch(event_id: i32) {
    renderer::with_app(|app| {
        if let Some(app) = app {
            app.dispatch_raw_event(event_id);
        }
    });
}

pub mod prelude {
    pub use crate::css;
    pub use crate::elements as dioxus_elements;
    pub use crate::events::{self, LynxEvent, LynxEventData};
    pub use crate::lynx_sys;
    pub use crate::{CSS, LynxApp, LynxMutations, inline_image, launch, launch_with_stylesheet};
    pub use dioxus_core;
    pub use dioxus_core::{
        Attribute, Callback, Component, Element, ErrorBoundary, ErrorContext, Event, EventHandler,
        Fragment, HasAttributes, IntoDynNode, RenderError, Result, ScopeId, SuspenseBoundary,
        SuspenseContext, VNode, VirtualDom, consume_context, provide_context, spawn, suspend,
        try_consume_context, use_drop, use_hook,
    };
    #[allow(deprecated)]
    pub use dioxus_core_macro::{Props, component, rsx};
    pub use dioxus_hooks::*;
    pub use dioxus_signals::{self, *};
}
