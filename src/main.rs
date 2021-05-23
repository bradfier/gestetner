use log::{debug, info};
use rand::Rng;
use std::fs::{DirEntry, File};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

mod http;

const HELP: &str = "\
Gestetner - A netcat pastebin

USAGE:
  gestetner -l '[::]:9999' -w '[::]:8080' -p /tmp/gst -u http://localhost:8080

FLAGS:
  -h, --help            Prints help information

OPTIONS:
  -u URL            Set the base URL to be returned in paste responses
  -l HOST:PORT      Set the listening socket address for incoming pastes
  -p PATH           Set the filesystem path in which to store pastes
  -w HOST:PORT      Set the listening socket for the HTTP server

  -n LENGTH         Set the length of the random paste slug (default: 4)
  -m MAX_SIZE       Set the maximum size of a paste in bytes (default: 512KiB)
  --capacity SIZE   Set the maximum size of the paste directory (default: 100MiB)
";

const DEFAULT_MAX_PASTE: usize = 524_288; // 512KiB
const DEFAULT_MAX_CAPACITY: usize = 104_857_600; // 100MiB

#[derive(Debug)]
struct Args {
    url: String,
    tcp_listen: std::net::SocketAddr,
    http_listen: std::net::SocketAddr,
    file_path: PathBuf,
    slug_length: usize,
    max_paste_size: usize,
    capacity: usize,
}

fn parse_args() -> Result<Args, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let args = Args {
        url: pargs.value_from_str("-u")?,
        tcp_listen: pargs.value_from_str("-l")?,
        http_listen: pargs.value_from_str("-w")?,
        file_path: pargs.value_from_str("-p")?,
        slug_length: pargs.value_from_str("-n").unwrap_or(4),
        max_paste_size: pargs.value_from_str("-m").unwrap_or(DEFAULT_MAX_PASTE),
        capacity: pargs
            .value_from_str("--capacity")
            .unwrap_or(DEFAULT_MAX_CAPACITY),
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

/// If the current directory size is bigger than `(capacity - new_file_size)`, delete the oldest files until there's room
fn maybe_prune_oldest(path: &Path, new_file_size: u64, capacity: u64) {
    let mut total: u64 = 0;
    let mut files: Vec<DirEntry> = std::fs::read_dir(path)
        .unwrap()
        .filter_map(|f| f.ok())
        .filter(|f| {
            if let Ok(file_type) = f.file_type() {
                file_type.is_file()
            } else {
                false
            }
        })
        .filter(|f| f.metadata().is_ok())
        .collect();

    files.sort_by_key(|f| {
        f.metadata()
            .unwrap()
            .created()
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });
    for f in files.iter() {
        if let Ok(meta) = f.metadata() {
            total += meta.len();
        }
    }

    while (total + new_file_size) >= capacity {
        let del = files.pop();
        if let Some(del) = del {
            debug!("Removing file {:?}", del.path());
            total -= del.metadata().unwrap().len();
            std::fs::remove_file(del.path()).expect("Failed to delete file");
        }
    }
}

pub(crate) fn create_paste(args: &Args, content: String) -> Result<String, std::io::Error> {
    let slug = random_slug(args.slug_length);
    let mut path = args.file_path.clone();
    path.push(slug.clone());
    maybe_prune_oldest(
        &args.file_path,
        content.as_bytes().len() as u64,
        args.capacity as u64,
    );
    File::create(path)?.write_all(content.as_bytes())?;
    Ok(format!("{}/{}", args.url, slug))
}

fn handle_paste(args: Arc<Args>, stream: TcpStream) -> Result<(), std::io::Error> {
    let (mut tx, rx) = (stream.try_clone().unwrap(), stream);
    // Read at most MAX_PASTE into a buffer, we just return a connection reset if the client tries to send more than that
    let mut buffer = Vec::with_capacity(args.max_paste_size);
    rx.set_read_timeout(Some(Duration::new(1, 0)))?;
    let read = rx.take(args.max_paste_size as u64).read_to_end(&mut buffer);

    if let Err(e) = read {
        if e.kind() != std::io::ErrorKind::WouldBlock {
            return Err(e);
        }
    }

    // Try and transform the buffer into a UTF-8 string and return an error to the client if that failed
    let text = String::from_utf8(buffer);
    match text {
        Ok(t) => {
            if !t.is_empty() {
                let url = create_paste(&args, t)?;
                tx.write_all(url.as_bytes())?;
                tx.write_all(b"\n")?;
            } else {
                tx.write_all(b"No content")?;
            }
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
    pretty_env_logger::init();

    // Create the storage directory if it doesn't exist
    std::fs::create_dir_all(&args.file_path).expect("Failed to create pastes directory");

    let http_args = args.clone();
    std::thread::spawn(move || http::serve(http_args));

    let socket = std::net::TcpListener::bind(args.tcp_listen).unwrap();
    info!("Paste socket listening on {}", socket.local_addr().unwrap());

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
