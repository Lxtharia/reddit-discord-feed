#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use dotenv::dotenv;
use reqwest;
use serde_json::json;
use minidom::Element;
use chrono::{DateTime};

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
    let _ = process_feed(client, &rss_url, &webhook_url, 1707255513).await;

}



async fn process_feed(client: reqwest::Client, reddit_url: &str, webhook_url: &str, time_last_post_send: i64) -> Result<(),reqwest::Error> {
    // Download the rss file and convert it to text
    let body: String = client.get(reddit_url).send()
        .await?
        .text()
        .await?;

    // parsing
    let posts = parse_xml(&body);
    let mut data = json!({});

    for post in posts.iter().rev() {

        // If the post was posted earlier than the last time we checked we shouldve processed it already
        if post.timestamp <= time_last_post_send {
            continue;
        }

        // Creating a json body to send to discord
        data = json!({
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
                        "value": format!("[{}]({})", post.author, post.author_url),
                    },
                ],
                "title": post.title,
                "url": post.url,
                "image": { "url": post.image_url },
            },
            ]
        });
        println!("POST: {:?}", data);
    }

    // Post json data to the discord webhook url
    let res = client.post(webhook_url)
        .json(&data)
        .send()
        .await?;

    Ok(())
}

#[derive(Clone)]
struct RedditPost {
    timestamp: i64,
    title: String,
    url: String,
    author: String,
    author_url: String,
    image_url: String,
}


fn parse_xml(body: &str) -> Vec<RedditPost> {

    println!("BODY: {:?}", body);

    let namespace = "http://www.w3.org/2005/Atom";
    let media_namespace = "http://search.yahoo.com/mrss/";
    let root: Element = body.parse().unwrap();

    let mut posts: Vec<RedditPost> = Vec::new();

    for trunk in root.children() {
        if trunk.is("entry", namespace) {

            // Defaults
            let mut timestamp = 0;
            let mut title = "[Kein Titel]".to_string();
            let mut url = "".to_string();
            let mut author = "u/?".to_string();
            let mut author_url = "".to_string();
            let mut image_url = "".to_string();

            // processing an entry
            for child in trunk.children() {

                if child.is("author", namespace) {
                    match child.get_child("name", namespace){
                        Some(elem) => author = elem.text(),
                        None => (),
                    };
                    match child.get_child("uri", namespace) {
                        Some(elem) => author_url = elem.text(),
                        None => (),
                    };
                } else if child.is("link", namespace) {
                    match child.attr("href"){
                        Some(elem) => url = elem.to_string(),
                        None => (),
                    };
                } else if child.is("published", namespace) {
                    let post_time_string = child.text();
                    timestamp = DateTime::parse_from_str(&post_time_string, "%Y-%m-%dT%H:%M:%S%z").unwrap().timestamp();

                } else if child.is("title", namespace) {
                    title = child.text();
                } else if child.is("thumbnail", media_namespace) {
                    match child.attr("url") {
                        Some(elem) => image_url = elem.to_string(),
                        None => (),
                    }
                }
            }

            // Add new object to list
            posts.push( 
                RedditPost {
                    timestamp: timestamp,
                    title: title,
                    url: url,
                    author: author,
                    author_url: author_url,
                    image_url: image_url,
                });

        }
    }

    return posts;
}

