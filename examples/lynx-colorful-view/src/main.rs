use dioxus_lynx::prelude::*;

const LEVEL1: [&str; 3] = ["77", "00", "ff"];
const LEVEL2: [&str; 16] = [
    "00", "11", "22", "33", "44", "55", "66", "77", "88", "99", "aa", "bb", "cc", "dd", "ee", "ff",
];
const LEVEL3: [&str; 16] = LEVEL2;
const LEVEL4: [&str; 8] = ["00", "11", "22", "33", "44", "55", "66", "77"];

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

fn color_style(red: &str, green: &str, blue: &str) -> String {
    format!("background-color: #{red}{green}{blue};")
}

#[allow(non_snake_case)]
fn App() -> Element {
    rsx! {
        view { class: "root",
            {LEVEL1.iter().map(|color1| {
                rsx! {
                    view {
                        class: "outer",
                        style: color_style(color1, color1, color1),
                        {LEVEL2.iter().map(move |color2| {
                            rsx! {
                                view {
                                    class: "block1",
                                    style: color_style(color1, color2, color2),
                                    {LEVEL3.iter().map(move |color3| {
                                        rsx! {
                                            view {
                                                class: "block2",
                                                style: color_style(color1, color2, color3),
                                                {LEVEL4.iter().map(move |color4| {
                                                    rsx! {
                                                        view {
                                                            class: "block3",
                                                            style: color_style(color2, color3, color4),
                                                        }
                                                    }
                                                })}
                                            }
                                        }
                                    })}
                                }
                            }
                        })}
                    }
                }
            })}
        }
    }
}

fn main() {
    launch_with_stylesheet(App, APP_STYLES);
}
