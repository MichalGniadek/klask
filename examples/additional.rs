//! Showcases additional input options
use clap::Parser;
use klask::Settings;
use std::io::{stdin, Read};

#[derive(Parser)]
struct Additional {
    /// Hides environment variables from output
    #[clap(long)]
    hide_environment_variables: bool,
    /// Hides stdin from output
    #[clap(long)]
    hide_stdin: bool,
    /// Hides working directory from output
    #[clap(long)]
    hide_working_directory: bool,
}

fn main() {
    let settings = Settings {
        enable_env: Some("Additional env description!".into()),
        enable_stdin: Some("Additional stdin description!".into()),
        // You don't have to provide a description
        enable_working_dir: Some("".into()),
        ..Default::default()
    };

    klask::run_derived::<Additional, _>(settings, |additional| {
        if !additional.hide_environment_variables {
            let v = std::env::vars().collect::<Vec<_>>();
            println!(
                "Environment variables: {:?} and {} more\n",
                &v[0..4],
                v.len() - 5
            );
        }

        if !additional.hide_stdin {
            println!("Stdin: {}\n", {
                let mut buf = String::new();
                stdin().read_to_string(&mut buf).unwrap();
                buf
            });
        }

        if !additional.hide_working_directory {
            println!("Directory: {:?}", std::env::current_dir().unwrap());
        }
    });
}
