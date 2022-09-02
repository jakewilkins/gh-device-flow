// #[macro_use]

extern crate clap;

use clap::Parser;

use github_device_flow::{authorize, refresh, Credential, DeviceFlowError};

/// GitHub Device Flow Authorizer
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
   /// Client ID
   #[clap(short, long, value_parser)]
   client_id: String,

   /// The host to authenticate with
   #[clap(short, long, value_parser)]
   host: Option<String>,

   /// A Refresh Token to exchange
   #[clap(short, long, value_parser)]
   refresh: Option<String>,
}

fn main() {
   let args = Args::parse();
   let cred: Result<Credential, DeviceFlowError>;

   match args.refresh {
       None => {
           cred = authorize(args.client_id, args.host);
       },
       Some(rt) => {
           cred = refresh(args.client_id.as_str(), rt.as_str(), args.host);
       }
   }
   match cred {
       Ok(cred) => {
           let json = serde_json::to_string_pretty(&cred).unwrap();
           println!("{}", json)
       },
       Err(err) => {
           println!("Something went wrong: {:?}", err)
       }
   }
}
