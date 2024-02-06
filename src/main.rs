use dotenv::dotenv;
use reqwest;
use serde_json::json;
use xml::reader::EventReader;

#[tokio::main]
async fn main() {

    // read feed and webhook url from .env file
    dotenv().ok();
    let rss_url = std::env::var("RSS_URL").expect("RSS_URL must be set");
    let webhook_url = std::env::var("WEBHOOK_URL").expect("WEBHOOK_URL must be set");

    // Create a http client
    let client = reqwest::Client::new();

    // process a feed, once
    let _ = process_feed(client, &rss_url, &webhook_url).await;

}


async fn process_feed(client: reqwest::Client, reddit_url: &str, webhook_url: &str) -> Result<(),reqwest::Error> {
    let body = client.get(reddit_url).send()
        .await?
        .text()
        .await?;

    // parsing
    let reader = EventReader::from_str(&body);
    parse_xml(&reader);

    let autor = "u/Maud-Lin";
    let autor_url = "https://www.reddit.com/r/schkreckl";
    let post_title = "Amazon??!";
    let post_url = "https://www.reddit.com/r/schkreckl/comments/7fhbvk/schkreckl/";
    let image_url = "https://i.redd.it/lzeskuzq96001.jpg";

    // Creating a json body to send to discord
    let data = json!({
        "username": "Schkreckl",
        "avatar_url": "https://styles.redditmedia.com/t5_4bnl6/styles/communityIcon_zimq8fp2clp11.png",
        "embeds": [
        {
            "color": 19608,
            "author": {
                "name": "Neuer Post auf Schkreckl!",
                "url": "https://www.reddit.com/r/schkreckl",
            },
            "fields": [
                {
                    "name": "Autor",
                    "value": format!("[{}]({})", autor, autor_url),
                },
            ],
            "title": post_title,
            "url": post_url,
            "image": { "url": image_url },
        },
        ]
    });

    let res = client.post(webhook_url)
        .json(&data)
        .send()
        .await?;

    Ok(())
}


fn parse_xml(reader: &EventReader<&[u8]>) -> () {

}

