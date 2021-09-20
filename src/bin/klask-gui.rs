#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::IntoApp;
use iced::{Application, Settings};
use klask::{example_opts::Opts, Klask};

fn main() {
    Klask::run(Settings::with_flags(Opts::into_app().bin_name("klask"))).unwrap();
}
