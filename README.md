# discourse-email-in-lambda

## Building

`rustup target install x86_64-unknown-linux-musl`

`cargo build --release --features vendored --target x86_64-unknown-linux-musl`

`zip -j lambda.zip ./target/x86_64-unknown-linux-musl/release/bootstrap`

## Setting up

This lambda takes 4 environment variables:
* `DISCOURSE_EMAIL_IN_BUCKET`: name of s3 bucket raw emails are placed in
* `DISCOURSE_URL`: base url of Discourse, without a trailing slash, eg: "https://discourse.mozilla.org"
* `DISCOURSE_API_KEY`
* `DISCOURSE_API_USERNAME`
* `REJECTED_RECIPIENTS`: comma separated list of recipients to not process email for

## Logging

This lambda uses [env_logger](https://docs.rs/env_logger/0.6.2/env_logger/) for logging, meaning you can change the logging level and style with environment variables.

By default:
* `RUST_LOG=info`
* `RUST_LOG_STYLE=never`
