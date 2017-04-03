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
struct LanguageServer {
    child_stdin: ChildStdin,
    pending: HashMap<usize, Callback>,
    _id: usize,
}


/// Generates a Language Server Protocol compliant message.
fn prepare_lsp_json(msg: &Value) -> Result<String, serde_json::error::Error> {
    let request = serde_json::to_string(&msg)?;
    Ok(format!("Content-Length: {}\n\r\n\r{}", request.len(), request))
}

impl LanguageServer {
    fn write(&mut self, msg: &str) {
        self.child_stdin.write_all(msg.as_bytes()).expect("error writing to stdin");
        self.child_stdin.write_all("\n".as_bytes()).expect("error writing to stdin");
        self.child_stdin.flush().expect("error flushing child stdin");
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
struct LanguageServerRef(Arc<Mutex<LanguageServer>>);

impl LanguageServerRef {
    fn new(child_stdin: ChildStdin) -> Self {
        LanguageServerRef(Arc::new(Mutex::new(LanguageServer {
            child_stdin: child_stdin,
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

impl Clone for LanguageServerRef {
    fn clone(&self) -> Self {
        LanguageServerRef(self.0.clone())
    }
}

fn run(mut child: Child) -> LanguageServerRef {
    let child_stdin = child.stdin.take().unwrap();
    let child_stdout = child.stdout.take().unwrap();
    let lang_server = LanguageServerRef::new(child_stdin);
    {
        let lang_server = lang_server.clone();
        thread::spawn(move ||{
            let reader = BufReader::new(child_stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => lang_server.handle_msg(&line),
                    Err(e) => panic!("error in read loop: {:?}", e),
                }
            }
        });
    }
    lang_server
}

fn main() {
    println!("starting main read loop");
    let lang_server = run(prepare_command());
    let reader = BufReader::new(stdin());
    for input in reader.lines() {
        match input {
            Ok(ref text) if text == "q" => break,
            Ok(text) => lang_server.write(&text),
            Err(_) => break,
        }
    } 
}


fn prepare_command() -> Child {
	Command::new("/bin/cat")
		.stdin(Stdio::piped())
		.stdout(Stdio::piped())
		.spawn()
		.expect("failed to start rls process")
}
