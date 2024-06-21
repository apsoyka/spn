mod config;
mod network;
mod panic;

use std::{env, path::PathBuf};

use config::{parse, setup_logging};
use panic::setup_panic;
use log::debug;
use network::submit;
use tokio::{fs::read_to_string, io::{stdin, AsyncReadExt}, main};

const API_ACCESS_KEY: &str = "API_ACCESS_KEY";
const API_SECRET_KEY: &str = "API_SECRET_KEY";
const API_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";

type BoxedError<'a> = Box<dyn std::error::Error + Send + Sync + 'a>;
type ReadResult<'a> = Result<Vec<String>, BoxedError<'a>>;
type MainResult<'a> = Result<(), BoxedError<'a>>;

async fn read<'a>(path: Option<PathBuf>) -> ReadResult<'a> {
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

#[main]
async fn main() -> MainResult<'static> {
    let arguments = parse();

    setup_panic();

    setup_logging(&arguments.verbosity)?;

    let client = reqwest::Client::builder()
        .connection_verbose(true)
        .user_agent(API_USER_AGENT)
        .build()?;

    let urls = read(arguments.input_file).await?;

    if urls.is_empty() { return Err("Nothing to do".into()); }

    if dotenv::dotenv().ok() == None { debug!("No dotfile found"); }

    let access_key = arguments.credentials.access_key.or(env::var(API_ACCESS_KEY).ok());
    let secret_key = arguments.credentials.secret_key.or(env::var(API_SECRET_KEY).ok());
    let keys = (access_key, secret_key);

    match keys {
        (Some(access_key), Some(secret_key)) => submit(&client, &urls, &access_key, &secret_key).await?,
        _ => panic!("Must provide an access key and a secret key")
    };

    Ok(())
}
