use std::{collections::{HashMap, VecDeque}, env, error::Error, path::PathBuf, process::exit};

use clap::{Args, Parser};
use log::{debug, error, info, warn, LevelFilter};
use reqwest::{Client, StatusCode};
use tokio::{fs::read_to_string, io::{stdin, AsyncReadExt}, time::{sleep, Duration}};

const API_ACCESS_KEY: &str = "API_ACCESS_KEY";
const API_SECRET_KEY: &str = "API_SECRET_KEY";
const API_URL: &str = "https://web.archive.org/save";
const API_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
const TIMEOUT_DURATION: Duration = Duration::from_secs(60);

#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = false)]
#[command(propagate_version = true)]
pub struct Arguments {
    #[command(flatten)]
    pub verbosity: Verbosity,

    #[command(flatten)]
    pub credentials: Credentials,

    /// A path to a file on the filesystem containing URLs
    input_file: Option<PathBuf>
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
    access_key: Option<String>,

    #[arg(short = 'S', long = "secret-key", help = "The secret key to use for authentication")]
    secret_key: Option<String>,
}

impl Verbosity {
    pub fn to_filter(&self) -> LevelFilter {
        if self.debug { LevelFilter::Trace }
        else if self.verbose { LevelFilter::Debug }
        else if self.quiet { LevelFilter::Warn }
        else { LevelFilter::Info }
    }
}

async fn read_urls(path: Option<PathBuf>) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
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

async fn submit_url(client: &Client, url: &str, access_key: &str, secret_key: &str) -> Result<(StatusCode, HashMap<String, String>), Box<dyn Error + Send + Sync>> {
    let mut form = HashMap::new();

    form.insert("url", url);
    form.insert("capture_all", "on");

    let token = format!("LOW {}:{}", access_key, secret_key);
    let response = client.post(API_URL)
        .header("Accept", "application/json")
        .header("Authorization", token)
        .form(&form)
        .send()
        .await?;
    let status = response.status();
    let json = response.json::<HashMap<String, String>>().await?;

    Ok((status, json))
}

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

async fn submit_urls(client: &Client, urls: &[String], access_key: &str, secret_key: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let count = urls.len();
    let mut queue = VecDeque::from_iter(urls.iter());
    let mut index: usize = 1;

    while let Some(url) = queue.pop_front() {
        let (status, response) = submit_url(client, url, access_key, secret_key).await?;

        debug!("\n{response:#?}");

        match status {
            StatusCode::OK => {
                info!("{index}/{count}: {status} -> {url}");

                if let Some(message) = response.get("message") { info!("{message}"); }

                index += 1;
            }
            StatusCode::TOO_MANY_REQUESTS => {
                warn!("{index}/{count}: {status} -> {url}");

                if let Some(message) = response.get("message") { warn!("{message}"); }

                // Wait for a set period of time.
                sleep(TIMEOUT_DURATION).await;

                // Put this URL back into the queue.
                queue.push_back(url);
            }
            _ => {
                error!("{index}/{count}: {status} -> {url}");

                if let Some(message) = response.get("message") { error!("{message}"); }

                index += 1;
            }
        };
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let arguments = Arguments::parse();

    setup_logging(&arguments.verbosity);

    let client = reqwest::Client::builder()
        .connection_verbose(true)
        .user_agent(API_USER_AGENT)
        .build()?;

    let urls = read_urls(arguments.input_file).await?;

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

    submit_urls(&client, &urls, &access_key.unwrap(), &secret_key.unwrap()).await?;

    Ok(())
}
