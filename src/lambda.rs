use env_logger::Env;
use failure::{format_err, Error};
use futures::future::{ok, Either};
use futures::{Future, Stream};
use lambda_runtime::{error::HandlerError, lambda, Context};
use log::info;
use regex::Regex;
use reqwest::r#async::Client;
use rusoto_s3::{GetObjectRequest, S3Client, S3};
use serde_json::{json, Value};
use std::env::var;
use tokio::runtime::Runtime;

fn main() {
    env_logger::from_env(
        Env::default()
            .default_filter_or("info")
            .default_write_style_or("never"),
    )
    .init();
    lambda!(my_handler);
}

fn my_handler(event: Value, ctx: Context) -> Result<(), HandlerError> {
    let s3_bucket = var("DISCOURSE_EMAIL_IN_BUCKET")?;
    let discourse_base_url = var("DISCOURSE_URL")?;
    let discourse_api_key = var("DISCOURSE_API_KEY")?;
    let discourse_api_username = var("DISCOURSE_API_USERNAME")?;
    let rejected_recipients = var("REJECTED_RECIPIENTS")?;

    let key = match &event["Records"][0]["ses"]["mail"]["messageId"] {
        Value::String(id) => id,
        _ => return Err(HandlerError::from("messageId isn't a string")),
    };
    info!("processing email with id {}", key);

    let dmarc_verdict = match &event["Records"][0]["ses"]["receipt"]["dmarcVerdict"]["status"] {
        Value::String(x) => x,
        _ => return Err(HandlerError::from("dmarcVerdict isn't a string")),
    };
    if dmarc_verdict == "FAIL" {
        info!("DMARC failed");
        info!("{}", &event);
        return Ok(());
    }

    let from_mozilla =
        Regex::new(r"@(mozilla\.com|getpocket\.com|mozillafoundation\.org|mozilla\.org)").unwrap();
    let from = match event["Records"][0]["ses"]["mail"]["commonHeaders"]["from"].as_array() {
        Some(x) => x,
        None => return Err(HandlerError::from("from isn't an array")),
    };
    for sender_value in from {
        let sender = match sender_value.as_str() {
            Some(x) => x,
            None => return Err(HandlerError::from("sender isn't a string")),
        };
        if from_mozilla.is_match(sender) && dmarc_verdict != "PASS" {
            info!("DMARC didn't pass for Mozilla domain");
            info!("{}", &event);
            return Ok(());
        }
    }

    let recipients = match event["Records"][0]["ses"]["receipt"]["recipients"].as_array() {
        Some(x) => x,
        None => return Err(HandlerError::from("recipients isn't an array")),
    };

    for rejected in rejected_recipients.split(',') {
        for recipient in recipients {
            if recipient == rejected {
                info!("recipient {} is in rejected list", recipient);
                return Ok(());
            }
        }
    }

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
                    .header("Api-Key", discourse_api_key)
                    .header("Api-Username", discourse_api_username)
                    .json(&json!({ "email": raw }))
                    .send()
                    .map_err(Error::from)
                    .and_then(|mut res| {
                        let status = res.status();
                        if status.is_success() {
                            Either::A(ok(res))
                        } else {
                            Either::B(
                                res.text()
                                    .map_err(Error::from)
                                    .and_then(move |text| {
                                        Err(format_err!("status: {}, body: {}", status, text))
                                    })
                                    .map_err(Error::from),
                            )
                        }
                    })
            })
            .map(|_| ())
            .map_err(HandlerError::from),
    )
}
