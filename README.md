# discourse-email-in-lambda

## Setting up

This lambda takes 4 environment variables:
* `DISCOURSE_EMAIL_IN_BUCKET`: name of s3 bucket raw emails are placed in
* `DISCOURSE_URL`: base url of Discourse, without a trailing slash, eg: "https://discourse.mozilla.org"
* `DISCOURSE_API_KEY`
* `DISCOURSE_API_USERNAME`
