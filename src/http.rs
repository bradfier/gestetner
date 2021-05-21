use log::info;
use rouille::Response;
use std::net::SocketAddr;
use std::path::PathBuf;

pub(crate) fn serve_pastes(addr: SocketAddr, path: PathBuf) {
    info!("Starting HTTP server on {}", addr);
    rouille::start_server(addr, move |request| {
        let response = rouille::match_assets(request, &path);

        if response.is_success() {
            response.with_unique_header("Content-Type", "text/plain; charset=UTF-8")
        } else {
            Response::text("Not Found").with_status_code(404)
        }
    })
}
