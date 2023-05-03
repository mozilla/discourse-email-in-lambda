use aws_lambda_events::event::ses::SimpleEmailEvent;
use aws_sdk_s3 as s3;
use env_logger::Env;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use log::info;
use regex::Regex;
use reqwest::Client;
use serde_json::json;
use std::env::var;

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::from_env(
        Env::default()
            .default_filter_or("info")
            .default_write_style_or("never"),
    )
    .init();

    run(service_fn(my_handler)).await
}

async fn my_handler(mut event: LambdaEvent<SimpleEmailEvent>) -> Result<(), Error> {
    let s3_bucket = var("DISCOURSE_EMAIL_IN_BUCKET")?;
    let discourse_base_url = var("DISCOURSE_URL")?;
    let discourse_api_key = var("DISCOURSE_API_KEY")?;
    let discourse_api_username = var("DISCOURSE_API_USERNAME")?;
    let rejected_recipients = var("REJECTED_RECIPIENTS")?;

    let ses = event
        .payload
        .records
        .pop()
        .ok_or::<Error>(Error::from("Missing records").into())?
        .ses;

    let key = ses
        .mail
        .message_id
        .ok_or(Error::from("messageId isn't a string"))?;

    info!("processing email with id {}", key);

    let dmarc_verdict = ses
        .receipt
        .dmarc_verdict
        .status
        .ok_or::<Error>(Error::from("dmarcVerdict isn't a string").into())?;

    if dmarc_verdict == "FAIL" {
        info!("DMARC failed");
        info!("{:?}", &event);
        return Ok(());
    }

    let spam_verdict = ses
        .receipt
        .spam_verdict
        .status
        .ok_or::<Error>(Error::from("Spam verdict isn't a string").into())?;

    if spam_verdict == "FAIL" {
        info!("SPAM failed");
        info!("{:?}", &event);
        return Ok(());
    }

    let virus_verdict = ses
        .receipt
        .virus_verdict
        .status
        .ok_or::<Error>(Error::from("Virus verdict isn't a string").into())?;

    if virus_verdict == "FAIL" {
        info!("VIRUS failed");
        info!("{:?}", &event);
        return Ok(());
    }

    let from_mozilla =
        Regex::new(r"@(mozilla\.com|getpocket\.com|mozillafoundation\.org|mozilla\.org)").unwrap();

    for sender in ses.mail.common_headers.from {
        if from_mozilla.is_match(&sender) && dmarc_verdict != "PASS" {
            info!("DMARC didn't pass for Mozilla domain");
            info!("{:?}", &event);
            return Ok(());
        }
    }

    for rejected in rejected_recipients.split(',') {
        for recipient in &ses.receipt.recipients {
            if recipient == rejected {
                info!("recipient {} is in rejected list", recipient);
                return Ok(());
            }
        }
    }

    let config = aws_config::load_from_env().await;
    let client = s3::Client::new(&config);
    let response = client
        .get_object()
        .bucket(s3_bucket)
        .key(key)
        .send()
        .await?;

    let body = response.body.collect().await?.to_vec();

    let client = Client::new();
    let url = discourse_base_url.to_owned() + "/admin/email/handle_mail";
    let res = client
        .post(&url)
        .header("Api-Key", discourse_api_key)
        .header("Api-Username", discourse_api_username)
        .json(&json!({ "email": body }))
        .send()
        .await?;

    let status = res.status();
    if status.is_success() {
        Ok(())
    } else {
        let text = res.text().await?;
        Err(anyhow::Error::msg(format!("status: {}, body: {}", status, text)).into())
    }
}
