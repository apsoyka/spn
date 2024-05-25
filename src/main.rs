use std::{collections::{HashMap, VecDeque}, error::Error, path::PathBuf};

use clap::Parser;
use log::{debug, info, warn, LevelFilter};
use reqwest::{Client, StatusCode};
use tokio::{fs::read_to_string, io::{stdin, AsyncReadExt}, time::{sleep, Duration}};

const API_URL: &str = "https://web.archive.org/save";
const API_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36";
const TIMEOUT_DURATION: Duration = Duration::from_secs(60);

#[derive(Parser)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
#[command(propagate_version = true)]
struct Arguments {
    access_key: String,

    secret_key: String,

    input_file: Option<PathBuf>
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

async fn submit_url(client: &Client, url: &str, access_key: &str, secret_key: &str) -> Result<StatusCode, Box<dyn Error + Send + Sync>> {
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

    debug!("\n{json:#?}");

    Ok(status)
}

fn setup_logging() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
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
        match submit_url(client, url, access_key, secret_key).await? {
            status @ StatusCode::OK => {
                info!("{index}/{count}: {status} -> {url}");

                index += 1;
            }
            status @ StatusCode::TOO_MANY_REQUESTS => {
                info!("{index}/{count}: {status} -> {url}");
                warn!("Rate limit hit; waiting...");

                // Wait for a minute.
                sleep(TIMEOUT_DURATION).await;

                // Put this URL back into the queue.
                queue.push_back(url);
            }
            status @ _ => {
                info!("{index}/{count}: {status} -> {url}");
                warn!("Skipping");

                index += 1;
            }
        };
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    setup_logging();

    let arguments = Arguments::parse();

    let client = reqwest::Client::builder()
        .connection_verbose(true)
        .user_agent(API_USER_AGENT)
        .build()?;

    let urls = read_urls(arguments.input_file).await?;

    submit_urls(&client, &urls, &arguments.access_key, &arguments.secret_key).await?;

    Ok(())
}
