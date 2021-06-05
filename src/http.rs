use crate::raterlimiter::ClientRateLimiter;
use crate::Args;
use log::info;
use rouille::{try_or_400, Response};
use std::cmp::min;
use std::io::Read;
use std::sync::Arc;

fn index_text(host: &str, url: &str) -> String {
    format!(
        "\
gestetner(1)                    GESTETNER                          gestetner(1)


NAME
    gestetner: a netcat and HTTP pastebin.

SYNOPSIS
    <command> | nc {host} 9999
    <command> | curl --data-binary @- {url}

EXAMPLES
    ~$ ls -l | curl --data-binary @- {url}
       {url}/abcd

    ~$ ls -l | nc {host} 9999
        {url}/efgh

SEE ALSO
    https://github.com/bradfier/gestetner

INSPIRED BY
    https://github.com/rupa/sprunge
    https://github.com/solusipse/fiche
",
        host = host,
        url = url
    )
}

pub(crate) fn serve(args: Arc<Args>, limiter: Arc<ClientRateLimiter>) {
    info!("Starting HTTP server on {}", args.http_listen);
    rouille::start_server(args.http_listen, move |request| match request.method() {
        "GET" => {
            if request.url() == "/" {
                return Response::text(index_text(&args.url_host(), &args.url));
            }
            let response = rouille::match_assets(request, &args.file_path);
            if response.is_success() {
                response.with_unique_header("Content-Type", "text/plain; charset=UTF-8")
            } else {
                Response::text("Not Found").with_status_code(404)
            }
        }
        "POST" => {
            if limiter.check_key(&request.remote_addr().ip()).is_err() {
                info!("Rate limited request from {}", &request.remote_addr().ip());
                return Response::text("Rate limited\n").with_status_code(429);
            }

            let content_length: Option<usize> = request
                .header("Content-Length")
                .and_then(|cl| cl.parse::<usize>().ok());
            let buf_size = if let Some(content_length) = content_length {
                min(content_length, args.max_paste_size)
            } else {
                args.max_paste_size
            };

            let mut buf = Vec::with_capacity(buf_size);
            let mut body = request.data().unwrap().take(buf_size as u64);
            body.read_to_end(&mut buf).expect("Failed to read body");

            let text = try_or_400!(String::from_utf8(buf));
            let url = crate::create_paste(&args, text).expect("Failed to create paste");

            let code = content_length
                .map(|cl| if cl > buf_size { 206 } else { 201 })
                .unwrap_or(201);

            Response::text(format!("{}\n", url))
                .with_unique_header("Location", url)
                .with_status_code(code)
        }
        _ => Response::empty_400().with_status_code(405),
    })
}
