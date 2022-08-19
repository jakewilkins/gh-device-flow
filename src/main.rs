// #[macro_use]

extern crate clap;

use clap::Parser;

use std::error::Error;

use github_device_flow::{authorize, refresh, Credential};

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
   let cred: Result<Credential, Box<dyn Error>>;

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
           println!("Your token is: {:?}", cred)
       },
       Err(err) => {
           println!("Something went wrong: {:?}", err)
       }
   }
}
