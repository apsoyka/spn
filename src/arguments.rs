use std::path::PathBuf;

use clap::{Args, Parser};
use log::LevelFilter;

#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = false)]
#[command(propagate_version = true)]
pub struct Arguments {
    #[command(flatten)]
    pub verbosity: Verbosity,

    #[command(flatten)]
    pub credentials: Credentials,

    /// A path to a file on the filesystem containing URLs
    pub input_file: Option<PathBuf>
}

#[derive(Args)]
#[group(multiple = false)]
pub struct Verbosity {
    #[arg(short = 'd', long = "debug", help = "Enable debugging output", global = true)]
    pub debug: bool,

    #[arg(short = 'v', long = "verbose", help = "Enable verbose output", global = true)]
    pub verbose: bool,

    #[arg(short = 'q', long = "quiet", help = "Suppress informational messages", global = true)]
    pub quiet: bool
}

#[derive(Args)]
#[group()]
pub struct Credentials {
    #[arg(short = 'A', long = "access-key", help = "The access key to use for authentication")]
    pub access_key: Option<String>,

    #[arg(short = 'S', long = "secret-key", help = "The secret key to use for authentication")]
    pub secret_key: Option<String>,
}

impl Verbosity {
    pub fn to_filter(&self) -> LevelFilter {
        if self.debug { LevelFilter::Trace }
        else if self.verbose { LevelFilter::Debug }
        else if self.quiet { LevelFilter::Warn }
        else { LevelFilter::Info }
    }
}
