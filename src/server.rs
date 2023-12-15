pub mod pb {
    tonic::include_proto!("grpc.examples.echo");
}

mod layers;
mod grpc_service;

use std::{
    net::ToSocketAddrs,
    pin::Pin,
    task::{ Context, Poll },

};
use tokio::runtime::Builder;
use tower::{Layer,Service};


// #[tokio::main]
// #[tokio::main(flavor = "multi_thread", worker_threads = 4)]
// #[tokio::main(flavor = "current_thread")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let rt = Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let layer = tower::ServiceBuilder
        ::new()
        .layer(tonic::service::interceptor(layers::layer1))
        .layer(tonic::service::interceptor(layers::layer2))
        .layer(MyMiddlewareLayer::default())
        .into_inner();

    rt.block_on(async {
        let server = grpc_service::EchoServer {};

        let _ = tonic::transport::Server
            ::builder()
            .layer(layer)
            .add_service(grpc_service::pb::echo_server::EchoServer::new(server))
            .serve("[::1]:50051".to_socket_addrs().unwrap().next().unwrap()).await;
    });

    Ok(())
}

#[derive(Debug, Clone, Default)]
struct MyMiddlewareLayer;

impl<S> Layer<S> for MyMiddlewareLayer {
    type Service = MyMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        MyMiddleware { inner: service }
    }
}

#[derive(Debug, Clone)]
struct MyMiddleware<S> {
    inner: S,
}

type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

impl<S> Service<tonic::codegen::http::request::Request<tonic::transport::Body>>
    for MyMiddleware<S>
    where
        S: Service<
            tonic::codegen::http::request::Request<tonic::transport::Body>,
            Response = tonic::codegen::http::Response<tonic::body::BoxBody>
        > +
            Clone +
            Send +
            'static,
        S::Future: Send + 'static
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(
        &mut self,
        req: tonic::codegen::http::request::Request<tonic::transport::Body>
    ) -> Self::Future {
        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary

        // This subtly breaks the contract. The service is driven to ready and then cloned before it is invoked. The original service is ready, but the clone is not necessarily ready.

        // To fix this, the call function could be rewritten as:

        //        Box::pin(self.0.call(req).err_into::<Error>())
        // which avoids the cloning.

        // If cloning is really necessary, you could use let mut inner = mem::replace(&mut self.inner self.0.clone()) to "take" the ready service and replace it with the clone.

        let mut clone = self.inner.clone();
        // let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let response = clone.call(req).await?;
            Ok(response)
        })
    }
}
