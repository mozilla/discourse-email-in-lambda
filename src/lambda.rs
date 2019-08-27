use failure::{format_err, Error};
use futures::Future;
use futures::Stream;
use lambda_runtime::{error::HandlerError, lambda, Context};
use reqwest::r#async::Client;
use rusoto_s3::{GetObjectRequest, S3Client, S3};
use serde_json::{json, Value};
use std::env::var;
use tokio::runtime::Runtime;

fn main() {
    lambda!(my_handler);
}

fn my_handler(event: Value, ctx: Context) -> Result<(), HandlerError> {
    let s3_bucket = var("DISCOURSE_EMAIL_IN_BUCKET")?;
    let discourse_base_url = var("DISCOURSE_URL")?;
    let discourse_api_key = var("DISCOURSE_API_KEY")?;
    let discourse_api_username = var("DISCOURSE_API_USERNAME")?;

    let key = match &event["Records"][0]["ses"]["mail"]["messageId"] {
        Value::String(id) => id,
        _ => return Err(HandlerError::from("messageId isn't a string")),
    };

    let request = GetObjectRequest {
        bucket: s3_bucket.to_string(),
        key: key.to_string(),
        ..Default::default()
    };

    let mut rt = Runtime::new().unwrap();

    let s3_client = S3Client::new(rusoto_core::Region::default());
    rt.block_on(
        s3_client
            .get_object(request)
            .map_err(Error::from)
            .and_then(|res| {
                res.body
                    .ok_or_else(|| format_err!("No body received from S3"))
            })
            .and_then(|body| body.concat2().map_err(Error::from))
            .map(|body| body.to_vec())
            .and_then(|body| String::from_utf8(body).map_err(Error::from))
            .and_then(move |raw| {
                let client = Client::new();
                let url = discourse_base_url.to_owned() + "/admin/email/handle_mail";
                client
                    .post(&url)
                    .query(&[("api_key", discourse_api_key)])
                    .query(&[("api_username", discourse_api_username)])
                    .json(&json!({ "email": raw }))
                    .send()
                    .map_err(|e| {
                        println!("hi {}", e);
                        Error::from(e)
                    })
            })
            .map(|_| ())
            .map_err(|e| HandlerError::from(e)),
    )
}
