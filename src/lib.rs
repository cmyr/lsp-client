#[macro_use]
extern crate serde_json;
extern crate jsonrpc_lite;

#[macro_use]
mod parsing;
pub mod client;

pub use client::{start_language_server, LanguageServerRef};
