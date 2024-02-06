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
    // Download the rss file and convert it to text
    let body: String = client.get(reddit_url).send()
        .await?
        .text()
        .await?;

    // parsing
    let (author, author_url, post_title, post_url, image_url) = parse_xml(&body);


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
                    "value": format!("[{}]({})", author, author_url),
                },
            ],
            "title": post_title,
            "url": post_url,
            "image": { "url": image_url },
        },
        ]
    });

    // Post json data to the discord webhook url
    let res = client.post(webhook_url)
        .json(&data)
        .send()
        .await?;

    Ok(())
}


fn parse_xml(body: &str) -> (String, String, String, String, String) {
    let reader = EventReader::from_str(&body);

    let author = "u/Maud-Lin";
    let author_url = "https://www.reddit.com/r/schkreckl";
    let post_title = "Amazon??!";
    let post_url = "https://www.reddit.com/r/schkreckl/comments/7fhbvk/schkreckl/";
    let image_url = "https://i.redd.it/lzeskuzq96001.jpg";
    return (author.to_string(), author_url.to_string(), post_title.to_string(), post_url.to_string(), image_url.to_string());
}

