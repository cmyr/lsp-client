#[macro_use]
extern crate serde_json;

mod parsing;

use std::sync::{Mutex, Arc};
use std::thread;
use std::process::{Command, Stdio, ChildStdin, Child};
use std::io::{Write, BufReader, BufRead, stdin};
use std::collections::HashMap;

use serde_json::value::Value;

// this to get around some type system pain related to callbacks. See:
// https://doc.rust-lang.org/beta/book/trait-objects.html,
// http://stackoverflow.com/questions/41081240/idiomatic-callbacks-in-rust
trait Callable: Send {
    fn call(self: Box<Self>, result: Result<Value, String>);
}

impl<F:Send + FnOnce(Result<Value, String>)> Callable for F {
    fn call(self: Box<F>, result: Result<Value, String>) {
        (*self)(result)
    }
}

type Callback = Box<Callable>;

/// Represents (and mediates communcation with) a Language Server.
///
/// LanguageServer should only ever be instantiated or accessed through an instance of
/// LanguageServerRef, which mediates access to a single shared LanguageServer through a Mutex.
struct LanguageServer<W: Write> {
    peer: W,
    pending: HashMap<usize, Callback>,
    _id: usize,
}


/// Generates a Language Server Protocol compliant message.
fn prepare_lsp_json(msg: &Value) -> Result<String, serde_json::error::Error> {
    let request = serde_json::to_string(&msg)?;
    Ok(format!("Content-Length: {}\r\n\r\n{}", request.len(), request))
}

impl <W:Write> LanguageServer<W> {
    fn write(&mut self, msg: &str) {
        self.peer.write_all(msg.as_bytes()).expect("error writing to stdin");
        self.peer.flush().expect("error flushing child stdin");
    }

    fn send_request(&mut self, method: &str, params: &Value, completion: Callback) {
        let request = json!({
            "jsonrpc": "2.0",
            "id": self._id,
            "method": method,
            "params": params
        });

        self.pending.insert(self._id, completion);
        self._id += 1;
        self.send_rpc(&request);
    }

    fn send_notification(&mut self, method: &str, params: &Value) {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        self.send_rpc(&notification);
    }

    fn send_rpc(&mut self, rpc: &Value) {
        let rpc = match prepare_lsp_json(&rpc) {
            Ok(r) => r,
            Err(err) => panic!("error encoding rpc {:?}", err),
        };
        self.write(&rpc);
    }
}

/// Access control and convenience wrapper around a shared LanguageServer instance.
struct LanguageServerRef<W: Write>(Arc<Mutex<LanguageServer<W>>>);

impl<W: Write> LanguageServerRef<W> {
    fn new(peer: W) -> Self {
        LanguageServerRef(Arc::new(Mutex::new(LanguageServer {
            peer: peer,
            pending: HashMap::new(),
            _id: 1,
        })))
    }

    /// Writes `msg` to the underlying process's stdin. Exposed for testing & debugging; 
    /// you should not need to call this method directly.
    fn write(&self, msg: &str) {
        let mut inner = self.0.lock().unwrap();
        inner.write(msg);
    }

    //TODO: actually parse and handle responses / notifications
    fn handle_msg(&self, msg: &str) {
        println!("server_ref handled '{}'", msg);
    }

    /// Sends a JSON-RPC request message with the provided method and parameters.
    /// `completion` should be a callback which will be executed with the server's response.
    fn send_request<CB>(&self, method: &str, params: &Value, completion: CB)
        where CB: 'static + Send + FnOnce(Result<Value, String>) {
            let mut inner = self.0.lock().unwrap();
            inner.send_request(method, params, Box::new(completion));
    }

    /// Sends a JSON-RPC notification message with the provided method and parameters.
    fn send_notification(&self, method: &str, params: &Value) {
        let mut inner = self.0.lock().unwrap();
        inner.send_notification(method, params);
    }
}

impl <W: Write> Clone for LanguageServerRef<W> {
    fn clone(&self) -> Self {
        LanguageServerRef(self.0.clone())
    }
}

fn run(mut child: Child) -> (Child, LanguageServerRef<ChildStdin>) {
    let child_stdin = child.stdin.take().unwrap();
    let child_stdout = child.stdout.take().unwrap();
    let lang_server = LanguageServerRef::new(child_stdin);
    {
        let lang_server = lang_server.clone();
        thread::spawn(move ||{
            let mut reader = BufReader::new(child_stdout);
            loop {
                match parsing::read_message(&mut reader) {
                    Ok(ref val) => println!("{:?}", serde_json::to_string(val)),
                    Err(err) => println!("parse error: {:?}", err),
                };
            }
        });
    }
    (child, lang_server)
}

fn main() {
    println!("starting main read loop");
    let (mut child, lang_server) = run(prepare_command());
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
