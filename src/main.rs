#[macro_use]
extern crate serde_json;
extern crate lsp_client;

use std::process::{Command, Stdio, Child};
use lsp_client::start_language_server;

fn main() {
    println!("starting main read loop");
    let (mut child, lang_server) = start_language_server(prepare_command());
    let init = json!({
        "process_id": "Null",
        "root_path": "/Users/cmyr/Dev/hacking/xi-mac/xi-editor",
        "initialization_options": {},
        "capabilities": {
            "documentSelector": ["rust"],
            "synchronize": {
                "configurationSection": "languageServerExample"
            }
        },
    });

    lang_server.send_request("initialize", &init, |result| {
        println!("received response {:?}", result);
    });
    child.wait();
}


fn prepare_command() -> Child {
    use std::env;
    let rls_root = env::var("RLS_ROOT").expect("$RLS_ROOT must be set");
        Command::new("cargo")
            .current_dir(rls_root)
            .args(&["run", "--release"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to start rls")
}
