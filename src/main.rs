use std::fs;
use std::error::Error;
use serde::{Serialize, Deserialize};
use serde_json::json;
use chrono::{DateTime};
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
    // Some optional values
    color: Option<u32>,
    title: Option<String>,
    title_url: Option<String>,
    webhook_user_name: Option<String>,
    webhook_avatar_url: Option<String>,
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

// Path to config file
const CONFIGFILE: &str = "config.toml";

fn load_config(filepath: &str) -> Result<Config, toml::de::Error> {
    let file_content = &fs::read_to_string(filepath).expect(&format!("There should be a configfile named '{}' in the current directory", CONFIGFILE));
    return toml::from_str(file_content);
}

fn write_config(filepath: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    let configfile_disclaimer = String::from(
r"# ====== INFO ======
# This configfile get's parsed, updated and written back by the program.
# Therefore, any comments and unused fields will get lost

");
    fs::write( filepath, configfile_disclaimer + toml::to_string(&config)?.as_str() )?;
    Ok(())
}

#[tokio::main]
async fn main() {

    // read feed- and webhook-url from config file
    let mut config = load_config(CONFIGFILE).unwrap();

    // Name your user agent after your app?
    static APP_USER_AGENT: &str = concat!(
        env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),
    );

    // Create a http client
    let http_client = reqwest::ClientBuilder::new()
        .user_agent(APP_USER_AGENT)
        .build().unwrap();

    // Process all the feeds and update the config after each one
    for i in 0..config.feeds.len() {
        let mut_feed = &mut config.feeds[i];
        println!("==== Processing feed [[ {} ]] =====", mut_feed.name);

        process_feed(&http_client, mut_feed).await.unwrap_or_else(|err| println!("Couldn't process feed. {}", err) );

        println!("Written new Config, updated timestamp: {}", &mut_feed.time_last_post_sent);
        write_config(CONFIGFILE, &config).unwrap_or_else(|err| println!("Couldn't write to config file. {}", err) );
    }

}


async fn process_feed(client: &reqwest::Client, feed: &mut Feed) -> Result<(),reqwest::Error> {
    
    // Download the rss file and convert it to text
    let body: String = client.get(&feed.rss_url).send()
        .await?
        .text()
        .await?;

    // parsing
    let mut posts = parse_atom_xml(&body);
    // Sort by newest
    posts.sort_by_key(|p| p.timestamp);

    for post in posts {
        // If the post was posted earlier than the latest post we posted, we assume we processed it already
        if post.timestamp <= feed.time_last_post_sent {
            continue;
        }

        // Creating a json body to send to discord
        let data = json!({
            "username": match &feed.webhook_user_name {
                Some(s) if s.is_empty() => None,
                Some(s) => Some(s),
                None => None },
            "avatar_url": feed.webhook_avatar_url,
            "embeds": [
            {
                "color": feed.color,
                "author": {
                    "name": feed.title,
                    "url": feed.rss_url,
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
        std::thread::sleep(std::time::Duration::from_millis(2000));

    }

    Ok(())
}


fn parse_atom_xml(body: &str) -> Vec<RedditPost> {

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

