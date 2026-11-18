//! Run with: cargo run --example async --features async -- <CLIENT_ID>

use github_device_flow::nonblocking::authorize;

#[tokio::main]
async fn main() {
    let client_id = std::env::args()
        .nth(1)
        .expect("usage: async <CLIENT_ID>");

    match authorize(client_id, None, None).await {
        Ok(cred) => println!("{}", serde_json::to_string_pretty(&cred).unwrap()),
        Err(err) => eprintln!("Something went wrong: {:?}", err),
    }
}
