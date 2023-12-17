use rsass::{compile_scss_path, output::Format};
use std::{fs, io, path::Path};

fn main() -> io::Result<()> {
    let scss_path = Path::new("src").join("style.scss");
    let css = compile_scss_path(&scss_path, Format::default()).expect("Failed to compile SCSS");

    let output_path = Path::new("assets").join("style.css");
    fs::write(output_path, css)?;

    Ok(())
}
