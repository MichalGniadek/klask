//! Showcases custom fonts
use clap::{Parser, ValueHint};
use klask::Settings;
use std::path::PathBuf;

#[derive(Parser)]
struct Font {
    #[clap(long, value_hint = ValueHint::AnyPath)]
    żółć: PathBuf,
}

fn main() {
    let settings = Settings {
        custom_font: Some(include_bytes!(r"font/Lato-Bold.ttf")),
        ..Default::default()
    };

    klask::run_derived::<Font, _>(settings, |_| {});
}
