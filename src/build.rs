use rsass::{compile_scss_path, output::Format};
use std::{fs::write, path::Path};

fn main() {
    let scss_path = Path::new("src").join("style.scss");
    let css = compile_scss_path(&scss_path, Format::default()).expect("Failed to compile SCSS");
    write(Path::new("src").join("style.css"), css).expect("Failed to write CSS");
}
