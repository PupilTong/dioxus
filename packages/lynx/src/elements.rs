#![allow(non_upper_case_globals)]

pub type AttributeDescription = (&'static str, Option<&'static str>, bool);

macro_rules! attr {
    ($name:ident) => {
        pub const $name: AttributeDescription = (stringify!($name), None, false);
    };
    ($name:ident => $value:literal) => {
        pub const $name: AttributeDescription = ($value, None, false);
    };
}

macro_rules! common_attrs {
    () => {
        attr!(class);
        attr!(style);
        attr!(id);
        attr!(src);
        attr!(clip_radius => "clip-radius");
        attr!(scroll_x => "scroll-x");
        attr!(scroll_y => "scroll-y");
        attr!(enable_scroll => "enable-scroll");
        attr!(enable_scroll_x => "enable-scroll-x");
        attr!(enable_scroll_y => "enable-scroll-y");
        attr!(bindtap);
    };
}

macro_rules! element {
    ($module:ident, $tag:literal) => {
        pub mod $module {
            use super::AttributeDescription;

            pub const TAG_NAME: &str = $tag;
            pub const NAME_SPACE: Option<&str> = None;

            common_attrs!();
        }
    };
}

element!(view, "view");
element!(text, "text");
element!(image, "image");
element!(scroll_view, "scroll-view");
element!(page, "page");
element!(list, "list");
element!(wrapper, "wrapper");

pub mod elements {
    pub use super::{image, list, page, scroll_view, text, view, wrapper};

    #[doc(hidden)]
    pub mod completions {
        #[allow(non_camel_case_types)]
        pub enum CompleteWithBraces {
            view {},
            text {},
            image {},
            scroll_view {},
            page {},
            list {},
            wrapper {},
        }
    }
}

pub mod extensions {}
pub mod traits {}

pub mod events {
    pub use crate::events::*;
}
