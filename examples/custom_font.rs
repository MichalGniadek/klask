//! Showcases custom fonts
use clap::{Parser, ValueHint};
use klask::Settings;
use std::{borrow::Cow, path::PathBuf};

#[derive(Parser)]
struct Font {
    #[clap(long, value_hint = ValueHint::AnyPath)]
    żółć: PathBuf,
}

fn main() {
    let mut settings = Settings::default();
    settings.custom_font = Some(Cow::Borrowed(include_bytes!(r"font/Lato-Bold.ttf")));

    klask::run_derived::<Font, _>(settings, |_| {});
}
