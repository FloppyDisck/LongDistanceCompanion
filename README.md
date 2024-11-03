# Long Distance Companion

## Generating Authentication Keys

Because im cheap and don't want to set up HTTPS certificates im going to use a different authentication method for POST
requests.
The main idea behind the concept is to simply sign the "transaction" with a secp256k1 PrivKey. The `.env` should have
the fields open.

## Running the server

Simply running `cargo run --package server --release` should do the trick

## Using the client

## Preparing the ESP32