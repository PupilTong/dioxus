use dioxus_lynx::prelude::*;

const STYLE: css::CSSTokenStream = CSS!(".root { color: red; }");
const INLINE: &str = inline_image!("tests/assets/pixel.png");

fn simple_app() -> Element {
    rsx! {
        view { class: "root",
            text { "hello" }
            image { src: INLINE }
        }
    }
}

fn event_app() -> Element {
    rsx! {
        view { ontap: move |_| {} }
    }
}

#[test]
fn css_macro_generates_token_stream() {
    assert!(STYLE.has_valid_header());
    assert!(STYLE.token_count().unwrap_or_default() > 0);
}

#[test]
fn inline_image_generates_data_url() {
    assert!(INLINE.starts_with("data:image/png;base64,"));
}

#[test]
fn rsx_uses_lynx_element_namespace() {
    let mut dom = VirtualDom::new(simple_app);
    let mutations = dom.rebuild_to_vec();

    assert!(
        mutations
            .edits
            .iter()
            .any(|mutation| matches!(mutation, dioxus_core::Mutation::LoadTemplate { .. }))
    );
    assert!(
        mutations
            .edits
            .iter()
            .any(|mutation| matches!(mutation, dioxus_core::Mutation::AppendChildren { .. }))
    );
}

#[test]
fn event_attribute_lowers_to_lynx_event_name() {
    let mut dom = VirtualDom::new(event_app);
    let mutations = dom.rebuild_to_vec();

    assert!(
        mutations.edits.iter().any(|mutation| {
            matches!(
                mutation,
                dioxus_core::Mutation::NewEventListener { name, .. } if name == "tap"
            )
        }),
        "mutations: {:?}",
        mutations.edits
    );
}
