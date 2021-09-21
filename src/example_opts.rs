use clap::Clap;

/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Debug, Clap)]
#[clap(version = "1.0", author = "author", name = "name")]
pub struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "default.conf")]
    config: String,
    /// Some input. Because this isn't an Option<T> it's required to be used
    input: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
}

#[derive(Debug, Clap)]
pub enum SubCommand {
    #[clap(version = "1.3", author = "Someone E. <someone_else@other.com>")]
    Test(Test),
    Run(Run),
    Walk(Walk),
}

/// A subcommand for controlling testing
#[derive(Debug, Clap)]
pub struct Test {
    /// Print debug info
    #[clap(short)]
    debug: bool,
}

/// A subcommand for running
#[derive(Debug, Clap)]
pub struct Run {
    /// Example
    lalala: String,
}

/// A subcommand for running2
#[derive(Debug, Clap)]
pub struct Walk {
    /// Example2
    #[clap(short)]
    lalala: String,
}
