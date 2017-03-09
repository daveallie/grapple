#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", deny(clippy_pedantic))]
#![cfg_attr(feature="clippy", allow(missing_docs_in_private_items))]

#![deny(missing_debug_implementations, missing_copy_implementations,
    trivial_casts, trivial_numeric_casts, unsafe_code,
    unused_import_braces, unused_qualifications)]

#[macro_use]
extern crate clap;
extern crate hyper;

use clap::App;
use hyper::Url;
use hyper::client::Client;
use hyper::method::Method;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

fn main() {
    let yaml = load_yaml!("cli.yml");
    let m = App::from_yaml(yaml)
        .version(VERSION)
        .get_matches();

    #[cfg_attr(feature="clippy", allow(option_unwrap_used))]
    let url = match Url::parse(m.value_of("uri").unwrap()) { // Unwrap is safe - required by clap
        Ok(uri) => uri,
        Err(e) => panic!("Couldn't parse URI: {}", e),
    };

    let client = Client::new();
    println!("{}", client.request(Method::Head, url).send().unwrap())
}
