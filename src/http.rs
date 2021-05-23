use crate::Args;
use log::info;
use rouille::{post_input, try_or_400, Response};
use std::sync::Arc;

pub(crate) fn serve(args: Arc<Args>) {
    info!("Starting HTTP server on {}", args.http_listen);
    rouille::start_server(args.http_listen, move |request| match request.method() {
        "GET" => {
            let response = rouille::match_assets(request, &args.file_path);
            if response.is_success() {
                response.with_unique_header("Content-Type", "text/plain; charset=UTF-8")
            } else {
                Response::text("Not Found").with_status_code(404)
            }
        }
        "POST" => {
            let input = try_or_400!(post_input!(request, {
                content: String,
            }));
            if !input.content.is_empty() {
                let url =
                    crate::create_paste(&args, input.content).expect("Failed to create paste");

                let mut body = url.clone();
                body.push('\n');
                Response::text(body).with_unique_header("Location", url)
            } else {
                Response::text("No content").with_status_code(400)
            }
        }
        _ => Response::empty_400().with_status_code(405),
    })
}
