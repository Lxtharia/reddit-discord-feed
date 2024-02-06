use reqwest;
use serde_json::json;

#[tokio::main]
async fn main() {
    // Create a http client
    let client = reqwest::Client::new();

    // process a feed, once
    let _ = process_feed(client, "https://www.reddit.com/r/schkreckl.rss", "https://discord.com/api/webhooks/894348357592559618/bCpUzEfUcZjcx2Gw4T28SQccWCpwrQzn7ssj8_rYJ-H278jZwfXDpBTubexkSMdSdxTe").await;

}


async fn process_feed(client: reqwest::Client, reddit_url: &str, webhook_url: &str) -> Result<(),reqwest::Error> {
    let body = client.get(reddit_url).send()
        .await?
        .text()
        .await?;

    // parsing

    // Creating a json body to send to discord
    let data = json!({
        "content" : "Hallo!"
    });

    let res = client.post(webhook_url)
        .json(&data)
        .send()
        .await?;

    Ok(())
}
