use lambda_runtime::{error::HandlerError, lambda, Context};
use serde_json::{Value, json};
use failure::{Error, format_err};
use reqwest::r#async::Client;
use rusoto_s3::{S3, S3Client, GetObjectRequest};
use futures::Future;
use futures::Stream;
use tokio::runtime::Runtime;
// use tokio::spawn;

fn main() {
    // let mut rt = Runtime::new().unwrap();
    lambda!(my_handler);
}

fn my_handler(event: Value, ctx: Context) -> Result<(), HandlerError> {
    println!("start of my_handler5");
    let s3_bucket = "discourse-staging-emailin";
    let discourse_base_url = "https://discourse-staging.production.paas.mozilla.community";
    let discourse_api_key = "";
    let discourse_api_username = "system";

    // let event: Value = serde_json::from_str(&data)?;
    let key = match &event["Records"][0]["ses"]["mail"]["messageId"] {
        Value::String(id) => id,
        _ => return Err(HandlerError::from("messageId isn't a string"))
    };

    let request = GetObjectRequest {
        bucket: s3_bucket.to_string(),
        key: key.to_string(),
        ..Default::default()
    };

    let mut rt = Runtime::new().unwrap();

    println!("s3_client");
    let s3_client = S3Client::new(rusoto_core::Region::default());
    rt.block_on(s3_client
        .get_object(request)
        .map_err(Error::from)
        .and_then(|res| {
            println!("1");
            res.body.ok_or_else(|| format_err!("No body received from S3"))
        })
        .and_then(|body| {
            println!("2");
            body.concat2().map_err(Error::from)
        })
        .map(|body| body.to_vec())
        .and_then(|body| String::from_utf8(body).map_err(Error::from))
        .and_then(move |raw| {
            println!("client");
            let client = Client::new();
            let url = discourse_base_url.to_owned() + "/admin/email/handle_mail";
            client.post(&url)
                .query(&[("api_key", discourse_api_key)])
                .query(&[("api_username", discourse_api_username)])
                .json(&json!({
                    "email": raw
                }))
                .send()
                .map_err(|e| {
                    println!("hi {}", e);
                    Error::from(e)
                })
        })
        .map(|_| {
            println!("5");
            ()
        })
        .map_err(|e| {
            println!("6");
            HandlerError::from(e)
        }))

    // println!("before shutdown on idle");
    // rt.shutdown_on_idle().wait().unwrap();
    // println!("after shutdown on idle");

    // println!("end of my_handler");
    // Ok(())
}
