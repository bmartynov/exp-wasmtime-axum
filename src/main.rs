mod wasm;

use axum::{
    Router,
    Extension,
    body::Body,
    http::Request,
    extract::Path,
    routing::post,
    response::{
        IntoResponse,
    },
};

use tower::Service;

use axum_macros::debug_handler;

#[tokio::main]
async fn main() {
    let wasm = wasm::Wasm::new().expect("maker");
    let svc = wasm::WasmSvc::new(wasm);

    // build our application with a single route
    let app = Router::new().
        route("/run/:module", post(run_module)).
        layer(Extension(svc));

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}


#[debug_handler]
async fn run_module(Path(module): Path<String>, Extension(mut wsvc): Extension<wasm::WasmSvc>, request: Request<Body>) -> impl IntoResponse {
    let (_, body) = request.into_parts();

    // this wont work if the body is an long running stream
    let bytes = hyper::body::to_bytes(body).
        await.
        expect("body");

    let response = wsvc.call((module, bytes)).
        await.
        expect("response");

    response
}
