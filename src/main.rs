mod arguments;

mod network;

use std::{env, error::Error, path::PathBuf, process::exit};

use arguments::{Arguments, Verbosity};
use clap::Parser;
use log::{debug, error};
use network::submit;
use tokio::{fs::read_to_string, io::{stdin, AsyncReadExt}};

const API_ACCESS_KEY: &str = "API_ACCESS_KEY";
const API_SECRET_KEY: &str = "API_SECRET_KEY";
const API_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";

type ReadResult = Result<Vec<String>, Box<dyn Error + Send + Sync>>;
type MainResult = Result<(), Box<dyn Error + Send + Sync>>;

fn setup_logging(verbosity: &Verbosity) {
    let filter = verbosity.to_filter();

    env_logger::builder()
        .filter_level(filter)
        .format_level(true)
        .format_target(false)
        .format_module_path(false)
        .format_timestamp_secs()
        .parse_default_env()
        .init();
}

async fn read(path: Option<PathBuf>) -> ReadResult {
    match path {
        Some(path) => {
            let buffer = read_to_string(path).await?;
            let output = buffer.split('\n')
                .filter_map(|value| (!value.trim().is_empty()).then_some(value.to_owned()))
                .collect();

            Ok(output)
        }
        None => {
            let mut stdin = stdin();
            let mut buffer = String::new();

            stdin.read_to_string(&mut buffer).await?;

            let output = buffer.split('\n')
                .filter_map(|value| (!value.trim().is_empty()).then_some(value.to_owned()))
                .collect();

            Ok(output)
        }
    }
}

#[tokio::main]
async fn main() -> MainResult {
    let arguments = Arguments::parse();

    setup_logging(&arguments.verbosity);

    let client = reqwest::Client::builder()
        .connection_verbose(true)
        .user_agent(API_USER_AGENT)
        .build()?;

    let urls = read(arguments.input_file).await?;

    if urls.is_empty() {
        error!("Nothing to do; quitting");

        exit(1);
    }

    if dotenv::dotenv().ok() == None {
        debug!("Failed to load credentials from dotfile");
    }

    let access_key = arguments.credentials.access_key.or(env::var(API_ACCESS_KEY).ok());
    let secret_key = arguments.credentials.secret_key.or(env::var(API_SECRET_KEY).ok());

    if access_key.is_none() || secret_key.is_none() {
        error!("Must provide an access key and secret key");

        exit(1);
    }

    submit(&client, &urls, &access_key.unwrap(), &secret_key.unwrap()).await?;

    Ok(())
}
