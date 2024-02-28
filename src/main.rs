use std::fs;
use std::error::Error;
use std::path::{Path, PathBuf};
use serde::{Serialize, Deserialize};
use serde_json::json;
use chrono::DateTime;
use minidom::{Element, NSChoice};
use reqwest;
use regex::Regex;

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
    save_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct RedditPost {
    timestamp: i64,
    title: String,
    url: String,
    thumbnail_url: Option<String>,
    image_url: Option<String>,
    author: Option<String>,
    author_url: Option<String>,
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

    println!("============ RUNNING ============");
    // Process all the feeds and update the config after each one
    for i in 0..config.feeds.len() {
        let mut_feed = &mut config.feeds[i];
        println!("====>> Processing feed [[ {} ]] == Saving images to: {:?} ", mut_feed.name, mut_feed.save_path);

        process_feed(&http_client, mut_feed).await.unwrap_or_else(|err| println!("Couldn't process feed. {}", err) );

        println!("Written new Config, updated timestamp: {}", &mut_feed.time_last_post_sent);
        write_config(CONFIGFILE, &config).unwrap_or_else(|err| println!("Couldn't write to config file. {}", err) );
    }
}


async fn process_feed(client: &reqwest::Client, feed: &mut Feed) -> Result<(), Box<dyn Error>> {

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

        // Format author string
        let post_author_string = match (&post.author, &post.author_url) {
            (Some(name), Some(url)) => format!("[{}]({})", name, url),
            (Some(name), None) => name.to_string(),
            _ => "[Unknown]".to_string(),
        };

        // Choose existing image/thumbnail url
        let embed_img_url: String = match (&post.thumbnail_url, &post.image_url) {
            (None, Some(iu) ) => iu.to_string(),
            (Some(tu), None) => tu.to_string(),
            _ => String::from(""),
        };

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
                    "url": feed.title_url,
                },
                "fields": [
                    {
                        "name": "Author",
                        "value": post_author_string,
                        "inline": true,
                    },
                    // TODO: Make optional
                    {
                        "name": "Time posted",
                        "value": format!("<t:{0}:D>\n<t:{0}:t>", post.timestamp),
                        "inline": true,
                    },
                ],
                "title": post.title,
                "url": post.url,
                "image": { "url": embed_img_url },
            },
            ]
        });

        println!("\t----- Sending post:\n\t{:?}", post);

        // Post json data to the discord webhook url
        let res = client.post(&feed.webhook_url)
            .json(&data)
            .send()
            .await?; // This exits the function on error (for example if the url is invalid)

        if res.status().is_success() {
            // Change the timestamp in the feed object
            feed.time_last_post_sent = post.timestamp;
        }

        // if a path to save to is given
        // TODO: check if file path is a valid (no file)  when loading config
        // and writable directory (Will break anyway, if its deleted in between)
        match (&feed.save_path, &post.image_url) {
            (Some(dst_path), Some(url)) => {
                let original_filestem: String = Path::new(&url).file_stem().unwrap().to_str().unwrap().to_string();
                let original_extention: String = Path::new(&url).extension().unwrap().to_str().unwrap().to_string();
                let filename = format!("{} - [{}].{}", &post.title, &original_filestem, original_extention);

                print!("\t\t----- Downloading Image to: {}/{}\n\t\t\t=> ", dst_path.display(), sanitize_filename(&filename)); // TODO: Not the real pathname
                match save_image(&client, dst_path, url, &filename).await {
                    Ok(_) => println!("Success!"),
                    Err(e) => println!("Error! {}", e),
                };
            },
            (Some(_), None) => println!("No image url found"),
            _ => (),
        };

        // Wait a bit to prevent getting rate limited
        std::thread::sleep(std::time::Duration::from_millis(2000));

    }

    Ok(())
}


fn parse_atom_xml(body: &str) -> Vec<RedditPost> {

    let root: Element = body.parse().unwrap();

    let mut namespace = NSChoice::Any;
    let mut media_namespace = NSChoice::Any;

    // Get namespaces
    if root.has_child("feed", namespace){
        namespace = root.get_child("feed", NSChoice::Any ).unwrap().attr("xmlns").unwrap_or("http://www.w3.org/2005/Atom").into();
        media_namespace = root.get_child("feed", NSChoice::Any).unwrap().attr("xmlns:media").unwrap_or("http://search.yahoo.com/mrss/").into();
    }

    let mut posts: Vec<RedditPost> = Vec::new();

    for trunk in root.children() {
        if trunk.is("entry", namespace) {

            // Defaults
            let mut timestamp = 0;
            let mut title = "[Kein Titel]".to_string();
            let mut url = "".to_string();
            let mut thumbnail_url = None;
            let mut image_url = None;
            let mut author = None;
            let mut author_url = None;

            // processing an entry
            for child in trunk.children() {

                if child.is("author", namespace) {
                    match child.get_child("name", namespace){
                        Some(elem) => author = Some(elem.text()),
                        None => (),
                    };
                    match child.get_child("uri", namespace) {
                        Some(elem) => author_url = Some(elem.text()),
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
                        Some(elem) => thumbnail_url = Some(elem.to_string()),
                        None => (),
                    }
                } else if child.is("content", namespace) {
                    let content = child.text();
                    // Read image url from content
                    let re = Regex::new(r"https://(i.redd.it|i.imgur.com)/.+\.(jpg|jpeg|png|webp|gif)").unwrap();
                    let caps = re.captures(&content);
                    if caps.is_some() {
                        image_url = match caps.unwrap().get(0) {
                            Some(cap_match) => Some(cap_match.as_str().to_string()),
                            None => None,
                        }
                    }
                }
            }

            // Add new object to list
            posts.push(
                RedditPost {
                    timestamp,
                    title,
                    url,
                    author,
                    author_url,
                    thumbnail_url,
                    image_url,
                });

        }
    }

    return posts;
}

async fn save_image(client: &reqwest::Client, dst_dir: &PathBuf, img_url: &String, filename: &str) -> Result<(), Box<dyn Error>> {
    let img_bytes = client.get(img_url).send()
        .await?
        .bytes()
        .await?;

    // write
    let mut full_path = dst_dir.clone();
    full_path.push(sanitize_filename(&filename));
    std::fs::write(full_path, img_bytes)?;

    Ok(())

}

fn sanitize_filename(filename: &str) -> String {
    let illegal_chars: Vec<char> = vec!['<', '>', ':', '"', '/', '\\', '|', '?', '*'];

    let result: String = filename
        .chars()
        .map(|c|
             if c == '"' { '\'' }
             else if illegal_chars.contains(&c) { '_' }
             else { c }
           )
        .collect();

    result
}

