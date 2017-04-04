//MIT License

//Copyright (c) 2017 Colin Rothfels

//Permission is hereby granted, free of charge, to any person obtaining a copy
//of this software and associated documentation files (the "Software"), to deal
//in the Software without restriction, including without limitation the rights
//to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//copies of the Software, and to permit persons to whom the Software is
//furnished to do so, subject to the following conditions:

//The above copyright notice and this permission notice shall be included in all
//copies or substantial portions of the Software.

//THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//SOFTWARE.

#[macro_use]
extern crate serde_json;
extern crate lsp_client;

use std::process::{Command, Stdio, Child};
use lsp_client::start_language_server;

fn main() {
    println!("starting main read loop");
    let (mut child, lang_server) = start_language_server(prepare_command());
    // this init blob was copied from the atom client example here:
    // https://github.com/jonathandturner/rls_vscode/blob/master/src/extension.ts
    let init = json!({
        "process_id": "Null",
        "root_path": "/Users/cmyr/Dev/hacking/xi-mac/xi-editor", // a path to some rust project
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
