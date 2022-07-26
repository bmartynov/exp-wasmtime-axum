mod wasm;
mod file;

use std::io;
use std::sync::Arc;

use axum::{
    Router,
    Extension,
    body::{
        Body,
        StreamBody,
    },
    http::Request,
    extract::Path,
    routing::post,
    response::IntoResponse,
};

use tokio::sync::mpsc::channel;

use crate::file::{StdinStream, StdoutChan};

#[tokio::main]
async fn main() {
    #[cfg(debug_assertions)]
    console_subscriber::init();

    let wasm = wasm::Wasm::new().expect("maker");

    // build our application with a single route
    let app = Router::new().
        route("/run/:module", post(run_module)).
        layer(Extension(Arc::new(wasm)));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn run_module(
    Extension(wsvc): Extension<Arc<wasm::Wasm>>,
    Path(module): Path<String>,
    request: Request<Body>,
) -> impl IntoResponse {
    let (_, body) = request.into_parts();

    let (stdout_tx, stdout_rx) = channel(10);
    let err_tx = stdout_tx.clone();

    let stdin = StdinStream::new(body);
    let stdout = StdoutChan::new(stdout_tx);

    tokio::spawn(async move {
        if let Err(err) = wsvc.run(module, Box::new(stdin), Box::new(stdout)).await {
            err_tx.send(Err(io::Error::new(io::ErrorKind::Other, err))).await.expect("ok");
        }
    });

    let stdout_stream = tokio_stream::wrappers::ReceiverStream::new(stdout_rx);

    StreamBody::new(stdout_stream)
}
