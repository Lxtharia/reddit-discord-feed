#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

use serde::{Serialize, Deserialize};
use serde_json::json;
use chrono::{DateTime};
use toml::Table;
use dotenv::dotenv;
use minidom::Element;
use reqwest;

#[derive(Deserialize, Serialize, Clone, Debug)]
struct Config {
    feeds: Vec<Feed>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct Feed {
    name: String,
    rss_url: String,
    webhook_url: String,
    time_last_post_sent: i64,
}

#[derive(Clone, Debug)]
struct RedditPost {
    timestamp: i64,
    title: String,
    url: String,
    author: String,
    author_url: String,
    image_url: String,
}

fn load_config(filename: &str) -> Config {
    let config: Config = toml::from_str(r#"
        [[feeds]]
        name = "Schkreckl"
        rss_url = "https://www.reddit.com/r/schkreckl.rss?sort=new"
        webhook_url = "https://discord.com/api/webhooks/894348357592559618/bCpUzEfUcZjcx2Gw4T28SQccWCpwrQzn7ssj8_rYJ-H278jZwfXDpBTubexkSMdSdxTe"
        time_last_post_sent = 1707305510
    "#).unwrap();

    return config;
}

#[tokio::main]
async fn main() {

    // read feed and webhook url from config file
    let mut config = load_config("config.toml");

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

    // Process all the feeds and update the config after each one
    for i in 0..config.feeds.len() {
        let oldtoml = toml::to_string(&config.clone()).unwrap();

        let mut_feed = &mut config.feeds[i];
        println!("==== Processing feed [[ {} ]] =====", mut_feed.name);

        println!("OLD TOML: {}", oldtoml);

        match process_feed(&client, mut_feed).await {
            Err(err) => { println!("Couldn't process feed. {}", err); }
            _ => ()
        }

        let newtoml = toml::to_string(&config).unwrap();
        println!("NEW TOML: {}", newtoml);
    }

}


async fn process_feed(client: &reqwest::Client, feed: &mut Feed) -> Result<(),reqwest::Error> {
    // Download the rss file and convert it to text
    let body: String = client.get(&feed.rss_url).send()
        .await?
        .text()
        .await?;

    // parsing
    let posts = parse_xml(&body);

    for post in posts.iter().sort_by_key(|p| p.timestamp ).rev() {

        // If the post was posted earlier than the last time we checked we shouldve processed it already
        if post.timestamp <= feed.time_last_post_sent {
            continue;
        }

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
                        "value": format!("[{}]({})", post.author, post.author_url),
                    },
                ],
                "title": post.title,
                "url": post.url,
                "image": { "url": post.image_url },
            },
            ]
        });

        println!("----- Sending post:\n {:?}", post);
        println!("-----");

        // Post json data to the discord webhook url
        let res = client.post(&feed.webhook_url)
            .json(&data)
            .send()
            .await?; // This exits the function on error (for example if the url is invalid)

        if res.status().is_success() {
            // Change the timestamp in the feed object
            feed.time_last_post_sent = post.timestamp;
        }

        // Wait a bit to prevent getting rate limited
        std::thread::sleep(std::time::Duration::from_millis(1000));

    }

    Ok(())
}


fn parse_xml(body: &str) -> Vec<RedditPost> {

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

