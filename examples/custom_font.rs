//! Showcases custom fonts
use clap::{Parser, ValueHint};
use klask::Settings;
use std::path::PathBuf;

#[derive(Parser)]
struct Font {
    /// Hides environment variables from output
    #[clap(long, value_hint = ValueHint::AnyPath)]
    文件路径: PathBuf,
}

fn main() {
    let settings = Settings {
        custom_font: Some(include_bytes!(r"C:\WINDOWS\FONTS\ARIALUNI.TTF")),
        ..Default::default()
    };

    klask::run_derived::<Font, _>(settings, |_| {});
}
