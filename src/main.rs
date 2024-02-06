use reqwest

fn main() {
    // Create a http client
    let client = reqwest::Client.new()

    // process a feed, once
    process_feed(client, "", "");

}


fn process_feed(client: reqwest::Client, reddit_url: str, webhook_url: str) {
    


}
