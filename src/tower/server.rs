use hyper::service::{ make_service_fn, service_fn };
use hyper::{ Body, Request, Response, Server };
use tower::make::Shared;
use tower::Service;
use tower_http::middleware::DefaultHeaders;
use tower_http::trace::TraceLayer;

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    // Your request handling logic goes here
    let response = Response::new(Body::from("Hello, Tokio Tower HTTP Server!"));
    Ok(response)
}

fn make_service() -> Shared<
    DefaultHeaders<
        TraceLayer<
            impl Service<
                Request<Body>,
                Response = Response<Body>,
                Error = hyper::Error,
                Future = impl std::future::Future<Output = Result<Response<Body>, hyper::Error>>
            >
        >
    >
> {
    // Wrap the service with middleware (optional)
    let service = service_fn(handle_request);
    let service = DefaultHeaders::new().layer(TraceLayer::new_for_http()).layer(service);

    // Convert the service into a shared service
    Shared::new(service)
}

#[tokio::main]
async fn main() {
    // Create a MakeServiceFn to build our service
    let make_service = make_service_fn(|_conn| { make_service() });

    // Create a hyper Server using Tokio runtime
    let addr = ([127, 0, 0, 1], 8080).into();
    let server = Server::bind(&addr).serve(make_service);

    println!("Server listening on http://{}", addr);

    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
}
