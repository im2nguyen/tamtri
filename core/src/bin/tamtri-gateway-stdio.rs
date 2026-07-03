use std::io::{self, BufRead, Write};

use serde_json::Value;

#[tokio::main]
async fn main() {
    let Some(endpoint) = std::env::args().nth(1) else {
        eprintln!("usage: tamtri-gateway-stdio <http://127.0.0.1:port/mcp>");
        std::process::exit(2);
    };
    let client = reqwest::Client::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        match forward(&client, &endpoint, &line).await {
            Ok(Some(response)) => {
                if serde_json::to_writer(&mut stdout, &response).is_err() {
                    break;
                }
                if writeln!(stdout).is_err() || stdout.flush().is_err() {
                    break;
                }
            }
            Ok(None) => {}
            Err(err) => {
                eprintln!("tamtri-gateway-stdio: {err}");
            }
        }
    }
}

async fn forward(
    client: &reqwest::Client,
    endpoint: &str,
    line: &str,
) -> Result<Option<Value>, Box<dyn std::error::Error + Send + Sync>> {
    let request: Value = serde_json::from_str(line)?;
    let response = client.post(endpoint).json(&request).send().await?;
    if response.status().as_u16() == 202 || response.status().as_u16() == 204 {
        return Ok(None);
    }
    Ok(Some(response.json().await?))
}
