use clap::{Parser, ValueHint};
use klask::{Localization, Settings};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(name = "App name")]
pub struct LocalizationExample {
    required_field: String,
    #[clap(long)]
    optional_field: Option<String>,
    #[clap(long, default_value = "default value")]
    field_with_default: String,
    #[clap(long, parse(from_os_str), value_hint = ValueHint::AnyPath)]
    native_path_picker: Option<PathBuf>,
    #[clap(short, multiple_occurrences(true))]
    multiple_values: Vec<String>,
}

fn main() {
    let mut settings = Settings::default();
    settings.enable_env = Some("Additional env description!".into());
    settings.enable_stdin = Some("Additional stdin description!".into());
    settings.enable_working_dir = Some("Additional working dir description!".into());
    settings.localization = polish_localization_exaple();

    klask::run_derived::<LocalizationExample, _>(settings, |_| {})
}

fn polish_localization_exaple() -> Localization {
    let mut loc = Localization::default();
    loc.optional = "(Opcjonalne)".into();
    loc.select_file = "Wybierz plik...".into();
    loc.select_directory = "Wybierz folder...".into();
    loc.new_value = "Nowa wartość".into();
    loc.reset = "Wyczyść".into();
    loc.reset_to_default = "Przywróć domyślną".into();
    loc.error_is_required = ("Argument '".into(), "' jest wymagany".into());
    loc.arguments = "Argumenty".into();
    loc.env_variables = "Zmienne środowiskowe".into();
    loc.error_env_var_cant_be_empty = "Zmienna środowiskowa nie może być pusta".into();
    loc.input = "Wejście".into();
    loc.text = "Tekst".into();
    loc.file = "Plik".into();
    loc.working_directory = "Katalog roboczy".into();
    loc.run = "Uruchom".into();
    loc.kill = "Zakończ".into();
    loc.running = "Działa".into();
    loc
}
