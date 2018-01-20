extern crate mio;
extern crate mio_uds;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use mio::*;
use mio::unix::EventedFd;
use std::env;
use std::fs::File;
use std::io::{stdin, stdout, Read, Write, Error, ErrorKind, Result};
use std::os::unix::io::AsRawFd;
use mio_uds::UnixStream;
use std::path::PathBuf;

#[derive(Deserialize)]
struct SbtSocketInfo {
    uri: String
}

fn mio_run() -> Result<()> {
    let sbt_dir = match env::args().skip(1).next() {
        Some(dir) => PathBuf::from(dir),
        None => env::current_dir().unwrap()
    };
    let file = File::open(
        sbt_dir.join("project").join("target").join("active.json"))?;
    let socket_info: SbtSocketInfo = serde_json::from_reader(file)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    let socket_path = socket_info.uri.rsplit("local://").next().unwrap();

    const SOCKET: Token = Token(0);
    const STDIN: Token = Token(1);
    let poll = Poll::new().unwrap();

    let mut sock = UnixStream::connect(&socket_path)?;
    poll.register(&sock,
                  SOCKET,
                  Ready::readable(),
                  PollOpt::edge())?;
    let mut stdin = stdin();
    let mut stdout = stdout();
    poll.register(&EventedFd(&stdin.as_raw_fd()),
                  STDIN,
                  Ready::readable(),
                  PollOpt::edge())?;
    let mut events = Events::with_capacity(1024);
    let mut socket_read_buf = [0; 1024];
    let mut stdin_read_buf = [0; 1024];
    loop {
        poll.poll(&mut events, None)?;
        for event in events.iter() {
            match event.token() {
                SOCKET => {
                    loop {
                        match sock.read(&mut socket_read_buf[..]) {
                            Err(ref error) if error.kind() == ErrorKind::WouldBlock => {
                                break;
                            },
                            Err(e) => {
                                return Err(e);
                            },
                            Ok(rb) => {
                                let mut wb = 0;
                                while wb < rb {
                                    wb += stdout.write(&socket_read_buf[wb..rb])?;
                                }
                            }

                        }
                    }
                }
                STDIN => {
                    loop {
                        match stdin.read(&mut stdin_read_buf[..]) {
                            Err(ref error) if error.kind() == ErrorKind::WouldBlock => {
                                break;
                            },
                            Err(e) => {
                                return Err(e);
                            },
                            Ok(rb) => {
                                let mut wb = 0;
                                while wb < rb {
                                    wb += sock.write(&stdin_read_buf[wb..rb])?;
                                }
                            }

                        }
                    }
                }
                _ => unreachable!()
            }
        }
    }
}

fn main() {
    mio_run().unwrap();
}
