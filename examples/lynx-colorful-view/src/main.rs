extern crate dioxus_lynx_sys as dioxus_lynx;

use dioxus_lynx_macro::CSS;
use dioxus_lynx_sys::{css, raw};

const LEVEL1: [u32; 3] = [0x77, 0x00, 0xff];
const LEVEL2: [u32; 16] = [
    0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
];
const LEVEL3: [u32; 16] = LEVEL2;
const LEVEL4: [u32; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];

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

fn create_view(class_name: &str) -> i32 {
    let node = raw::create_view();
    raw::set_classes(node, class_name);
    node
}

#[inline(always)]
fn create_color_view(class_name: &str, red: u32, green: u32, blue: u32) -> i32 {
    let node = create_view(class_name);
    let style = format!("background-color:#{red:02x}{green:02x}{blue:02x};");
    raw::set_string_attribute(node, "style", &style);
    node
}

fn append_children(parent: i32, children: &[i32]) {
    raw::replace_elements(parent, children, &[], None);
}

fn colorful_view() -> i32 {
    let root = create_view("root");
    let mut outer_nodes = [0; LEVEL1.len()];

    for color1_index in 0..LEVEL1.len() {
        let color1 = LEVEL1[color1_index];
        let outer = create_color_view("outer", color1, color1, color1);
        let mut block1_nodes = [0; LEVEL2.len()];

        for color2_index in 0..LEVEL2.len() {
            let color2 = LEVEL2[color2_index];
            let block1 = create_color_view("block1", color1, color2, color2);
            let mut block2_nodes = [0; LEVEL3.len()];

            for color3_index in 0..LEVEL3.len() {
                let color3 = LEVEL3[color3_index];
                let block2 = create_color_view("block2", color1, color2, color3);
                let mut block3_nodes = [0; LEVEL4.len()];

                for color4_index in 0..LEVEL4.len() {
                    let color4 = LEVEL4[color4_index];
                    block3_nodes[color4_index] =
                        create_color_view("block3", color2, color3, color4);
                }

                append_children(block2, &block3_nodes);
                block2_nodes[color3_index] = block2;
            }

            append_children(block1, &block2_nodes);
            block1_nodes[color2_index] = block1;
        }

        append_children(outer, &block1_nodes);
        outer_nodes[color1_index] = outer;
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
