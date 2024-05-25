use std::{collections::{HashMap, VecDeque}, error::Error};

use log::{debug, error, info, warn};
use reqwest::{Client, StatusCode};
use tokio::time::{sleep, Duration};

const API_URL: &str = "https://web.archive.org/save";
const TIMEOUT_DURATION: Duration = Duration::from_secs(60);

type RequestResult = Result<(StatusCode, HashMap<String, String>), Box<dyn Error + Send + Sync>>;
type SubmitResult = Result<(), Box<dyn Error + Send + Sync>>;

async fn send(client: &Client, url: &str, access_key: &str, secret_key: &str) -> RequestResult {
    let token = format!("LOW {}:{}", access_key, secret_key);
    let mut form = HashMap::new();

    form.insert("url", url);
    form.insert("capture_all", "on");

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

pub async fn submit(client: &Client, urls: &[String], access_key: &str, secret_key: &str) -> SubmitResult {
    let count = urls.len();
    let mut queue = VecDeque::from_iter(urls.iter());
    let mut index: usize = 1;

    while let Some(url) = queue.pop_front() {
        let (status, response) = send(client, url, access_key, secret_key).await?;

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
