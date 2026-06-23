use dioxus_lynx::{CSS, css, lynx_sys::raw};

const LEVEL1: [&str; 3] = ["77", "00", "ff"];
const LEVEL2: [&str; 16] = [
    "00", "11", "22", "33", "44", "55", "66", "77", "88", "99", "aa", "bb", "cc", "dd", "ee", "ff",
];
const LEVEL3: [&str; 16] = LEVEL2;
const LEVEL4: [&str; 8] = ["00", "11", "22", "33", "44", "55", "66", "77"];
include!(concat!(env!("OUT_DIR"), "/color_styles.rs"));

const APP_STYLES: css::CSSTokenStream = CSS!(
    r#"
.root {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
}

.outer {
  margin: 1px;
  height: 100%;
  display: flex;
  flex-direction: row;
}

.block1 {
  margin: 1px;
  width: 100%;
  display: flex;
  flex-direction: column;
}

.block2 {
  margin: 1px;
  height: 100%;
  display: flex;
  flex-wrap: wrap;
  flex-direction: row;
  justify-content: center;
  align-content: center;
  align-items: center;
}

.block3 {
  width: 15%;
  height: 20%;
  margin: 1px;
}
"#
);

fn create_view(class_name: &str, style: Option<&'static str>) -> i32 {
    let node = raw::create_view();
    raw::set_classes(node, class_name);
    if let Some(style) = style {
        raw::set_string_attribute(node, "style", style);
    }
    node
}

fn append_children(parent: i32, children: &[i32]) {
    raw::replace_elements(parent, children, &[], None);
}

fn colorful_view() -> i32 {
    let root = create_view("root", None);
    let mut outer_nodes = Vec::with_capacity(LEVEL1.len());

    for (color1_index, _) in LEVEL1.iter().enumerate() {
        let outer = create_view("outer", Some(OUTER_STYLES[color1_index]));
        let mut block1_nodes = Vec::with_capacity(LEVEL2.len());

        for (color2_index, _) in LEVEL2.iter().enumerate() {
            let block1 = create_view("block1", Some(BLOCK1_STYLES[color1_index][color2_index]));
            let mut block2_nodes = Vec::with_capacity(LEVEL3.len());

            for (color3_index, _) in LEVEL3.iter().enumerate() {
                let block2 = create_view(
                    "block2",
                    Some(BLOCK2_STYLES[color1_index][color2_index][color3_index]),
                );
                let mut block3_nodes = Vec::with_capacity(LEVEL4.len());

                for (color4_index, _) in LEVEL4.iter().enumerate() {
                    block3_nodes.push(create_view(
                        "block3",
                        Some(BLOCK3_STYLES[color2_index][color3_index][color4_index]),
                    ));
                }

                append_children(block2, &block3_nodes);
                block2_nodes.push(block2);
            }

            append_children(block1, &block2_nodes);
            block1_nodes.push(block1);
        }

        append_children(outer, &block1_nodes);
        outer_nodes.push(outer);
    }

    append_children(root, &outer_nodes);
    root
}

fn main() {
    raw::replace_style_sheets_tokens(APP_STYLES);
    let page = raw::get_page_element().unwrap_or_else(raw::create_page);
    let root = colorful_view();
    raw::append_element(page, root);
    raw::flush_element_tree(None);
}
