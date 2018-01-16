extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::env;
use std::fs::File;
use std::io::{copy, stdin, stdout, Error, ErrorKind, Result};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::thread;

#[derive(Deserialize)]
struct SbtSocketInfo {
    uri: String
}

fn run() -> Result<()> {
    let sbt_dir = match env::args().skip(1).next() {
        Some(dir) => PathBuf::from(dir),
        None => env::current_dir().unwrap()
    };
    let file = File::open(
        sbt_dir.join("project").join("target").join("active.json"))?;
    let socket_info: SbtSocketInfo = serde_json::from_reader(file)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    let socket_path = socket_info.uri.rsplit("local://").next().unwrap();

    // Start up the listener
    let mut lsp_socket = UnixStream::connect(socket_path)?;
    lsp_socket.set_nonblocking(false)?;
    let mut write_half = lsp_socket.try_clone()?;

    let t1 = thread::spawn(move || {
        copy(&mut stdin(), &mut write_half).unwrap();
    });
    thread::spawn(move || {
        copy(&mut lsp_socket, &mut stdout()).unwrap();
    });
    let _ = t1.join();
    Ok(())
}

fn main() {
    run().unwrap();
}
