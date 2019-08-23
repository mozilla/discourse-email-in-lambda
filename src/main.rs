use std::fs;
use serde_json::{Value, json};
use failure::{Error, bail, format_err};
use reqwest::r#async::Client;
use rusoto_s3::{S3, S3Client, GetObjectRequest};
use futures::Future;
use futures::Stream;
use tokio::run;

fn main() {
    run(my_handler());
}

fn my_handler() -> impl Future<Item = (), Error = ()> {
    let s3_bucket = "";
    let discourse_base_url = "";
    let discourse_api_key = "";
    let discourse_api_username = "";

    let data = fs::read_to_string("event.json")
        .expect("Something went wrong reading the file");

    let event: Value = serde_json::from_str(&data).unwrap();
    let key = match &event["Records"][0]["ses"]["mail"]["messageId"] {
        Value::String(id) => id,
        _ => "messageId isn't a string"
    };

    let request = GetObjectRequest {
        bucket: s3_bucket.to_string(),
        key: key.to_string(),
        ..Default::default()
    };

    let s3_client = S3Client::new(rusoto_core::Region::default());
    println!("Hello World!");
    s3_client
        .get_object(request)
        .map_err(Error::from)
        .and_then(|res| res.body.ok_or_else(|| format_err!("No body received from S3")))
        .and_then(|body| body.concat2().map_err(Error::from))
        .map(|body| body.to_vec())
        .and_then(|body| String::from_utf8(body).map_err(Error::from))
        .and_then(move |raw| {
            let client = Client::new();
            let url = discourse_base_url.to_owned() + "/admin/email/handle_mail";
            client.post(&url)
                .query(&[("api_key", discourse_api_key)])
                .query(&[("api_username", discourse_api_username)])
                .json(&json!({
                    "email": raw
                }))
                .send()
                .map_err(Error::from)
        })
        .map(|_| ())
        .map_err(|_| ())
}
