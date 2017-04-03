use std::sync::{Mutex, Arc};
use std::thread;
use std::process::{Command, Stdio, ChildStdin, Child};
use std::io::{Write, BufReader, BufRead, stdin};


struct LanguageServer {
    child_stdin: ChildStdin
}

impl LanguageServer {
    fn send(&mut self, msg: &str) {
        self.child_stdin.write_all(msg.as_bytes());
        self.child_stdin.write_all("\n".as_bytes());
    }
}

struct LanguageServerRef(Arc<Mutex<LanguageServer>>);

impl LanguageServerRef {
    fn new(child_stdin: ChildStdin) -> Self {
        LanguageServerRef(Arc::new(Mutex::new(LanguageServer {
            child_stdin: child_stdin
        })))
    }

    fn send(&self, msg: &str) {
        let mut inner = self.0.lock().unwrap();
        inner.send(msg);
    }

    fn handle_msg(&self, msg: &str) {
        println!("server_ref handled '{}'", msg);
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
    let mut reader = BufReader::new(stdin());
    for input in reader.lines() {
        match input {
            Ok(ref text) if text == "q" => break,
            Ok(text) => lang_server.send(&text),
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
