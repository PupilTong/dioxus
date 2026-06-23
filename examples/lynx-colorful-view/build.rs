use std::env;
use std::fs;
use std::path::PathBuf;

const LEVEL1: [&str; 3] = ["77", "00", "ff"];
const LEVEL2: [&str; 16] = [
    "00", "11", "22", "33", "44", "55", "66", "77", "88", "99", "aa", "bb", "cc", "dd", "ee", "ff",
];
const LEVEL3: [&str; 16] = LEVEL2;
const LEVEL4: [&str; 8] = ["00", "11", "22", "33", "44", "55", "66", "77"];

fn style(red: &str, green: &str, blue: &str) -> String {
    format!("\"background-color: #{red}{green}{blue};\"")
}

fn main() {
    let mut out = String::new();

    out.push_str("const OUTER_STYLES: [&str; 3] = [");
    for color1 in LEVEL1 {
        out.push_str(&style(color1, color1, color1));
        out.push(',');
    }
    out.push_str("];\n");

    out.push_str("const BLOCK1_STYLES: [[&str; 16]; 3] = [");
    for color1 in LEVEL1 {
        out.push('[');
        for color2 in LEVEL2 {
            out.push_str(&style(color1, color2, color2));
            out.push(',');
        }
        out.push_str("],");
    }
    out.push_str("];\n");

    out.push_str("const BLOCK2_STYLES: [[[&str; 16]; 16]; 3] = [");
    for color1 in LEVEL1 {
        out.push('[');
        for color2 in LEVEL2 {
            out.push('[');
            for color3 in LEVEL3 {
                out.push_str(&style(color1, color2, color3));
                out.push(',');
            }
            out.push_str("],");
        }
        out.push_str("],");
    }
    out.push_str("];\n");

    out.push_str("const BLOCK3_STYLES: [[[&str; 8]; 16]; 16] = [");
    for color2 in LEVEL2 {
        out.push('[');
        for color3 in LEVEL3 {
            out.push('[');
            for color4 in LEVEL4 {
                out.push_str(&style(color2, color3, color4));
                out.push(',');
            }
            out.push_str("],");
        }
        out.push_str("],");
    }
    out.push_str("];\n");

    let path = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("color_styles.rs");
    fs::write(path, out).unwrap();
}
