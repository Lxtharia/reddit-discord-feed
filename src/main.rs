#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use dotenv::dotenv;
use reqwest;
use serde_json::json;
use minidom::Element;

#[tokio::main]
async fn main() {

    // read feed and webhook url from .env file
    dotenv().ok();
    let rss_url = std::env::var("RSS_URL").expect("RSS_URL must be set");
    let webhook_url = std::env::var("WEBHOOK_URL").expect("WEBHOOK_URL must be set");

    // Name your user agent after your app?
    static APP_USER_AGENT: &str = concat!(
        env!("CARGO_PKG_NAME"),
        "/",
        env!("CARGO_PKG_VERSION"),
    );

    // Create a http client
    let client = reqwest::ClientBuilder::new()
        .user_agent(APP_USER_AGENT)
        .build().unwrap();

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
    let (post_time, author, author_url, post_title, post_url, image_url) = parse_xml(&body);


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

    println!("{:?}", data);

    return Ok(());

    // Post json data to the discord webhook url
    let res = client.post(webhook_url)
        .json(&data)
        .send()
        .await?;

    Ok(())
}


fn parse_xml(body: &str) -> (u64, String, String, String, String, String) {

    println!("Body: {:?}", body);

    let namespace = "http://www.w3.org/2005/Atom";
    let root: Element = body.parse().unwrap();

    // Defaults?
    let mut post_time = 0;
    let mut author = "u/?".to_string();
    let mut author_url = "".to_string();
    let mut post_title = "[Titel]".to_string();
    let mut post_url = "".to_string();
    let mut image_url = "".to_string();

    for trunk in root.children() {
        if trunk.is("entry", namespace) {
            for child in trunk.children() {
                if child.is("author", namespace) {
                    author = child.get_child("name", namespace).unwrap().text();
                    author_url = child.get_child("uri", namespace).unwrap().text();
                } else if child.is("link", namespace) {
                    post_url = child.attr("href").unwrap().to_string();
                } else if child.is("published", namespace) {
                    let post_time_string = child.text();

                } else if child.is("title", namespace) {
                    post_title = child.text();
                } else if child.is("thumbnail", namespace) {
                    image_url = child.attr("url").unwrap().to_string();
                }
            }
        }
    }

    return (post_time, author, author_url, post_title, post_url, image_url);
}

