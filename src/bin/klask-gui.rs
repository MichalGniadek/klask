#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::IntoApp;
use eframe;
use klask::{example_opts::Opts, Klask};

fn main() {
    let app = Klask::new(Opts::into_app().bin_name("klask"));
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
}
