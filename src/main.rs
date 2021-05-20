use rand::Rng;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const HELP: &str = "\
Gestetner - A netcat pastebin

USAGE:
  gestetner --listen 127.0.0.1:9999

FLAGS:
  -h, --help            Prints help information

OPTIONS:
  -l HOST:PORT      Set the listening socket address for incoming pastes
  -p PATH           Set the filesystem path in which to store pastes
";

const MAX_PASTE: usize = 524_288; // 512KiB

#[derive(Debug)]
struct Args {
    listen: std::net::SocketAddr,
    file_path: PathBuf,
}

fn parse_args() -> Result<Args, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let args = Args {
        listen: pargs.value_from_str("-l")?,
        file_path: pargs.value_from_str("-p")?,
    };

    Ok(args)
}

fn random_slug(l: usize) -> String {
    let mut rng = rand::thread_rng();
    let mut out = String::with_capacity(l);
    for _ in 0..l {
        out.push(rng.gen_range(b'a'..b'z') as char);
    }
    out
}

fn handle_paste(args: Arc<Args>, stream: TcpStream) -> Result<(), std::io::Error> {
    let (mut tx, rx) = (stream.try_clone().unwrap(), stream);
    // Read at most MAX_PASTE into a buffer, we just return a connection reset if the client tries to send more than that
    let mut buffer = Vec::with_capacity(MAX_PASTE);
    rx.set_read_timeout(Some(Duration::new(1, 0)))?;
    let read = rx.take(MAX_PASTE as u64).read_to_end(&mut buffer);

    if let Err(e) = read {
        if e.kind() != std::io::ErrorKind::WouldBlock {
            return Err(e);
        }
    }

    // Try and transform the buffer into a UTF-8 string and return an error to the client if that failed
    let text = String::from_utf8(buffer);
    match text {
        Ok(ref t) => {
            let slug = random_slug(4);
            let mut path = args.file_path.clone();
            path.push(slug.clone());
            File::create(path)?.write_all(t.as_bytes())?;
            tx.write_all(format!("http://localhost:8080/{}\n", slug).as_bytes())?;
        }
        Err(_) => {
            tx.write_all(b"Failed to parse paste as UTF-8")?;
        }
    }

    Ok(())
}

fn main() {
    let args = parse_args();
    if let Err(e) = args {
        println!("{}", e);
        std::process::exit(1);
    }
    let args = Arc::new(args.unwrap());

    // Create the storage directory if it doesn't exist
    std::fs::create_dir_all(&args.file_path);

    let socket = std::net::TcpListener::bind(args.listen).unwrap();

    for stream in socket.incoming() {
        match stream {
            Ok(s) => {
                let inner_args = args.clone();
                std::thread::spawn(move || handle_paste(inner_args, s));
            }
            Err(e) => println!("Error connecting: {}", e),
        }
    }
}
