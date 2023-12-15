pub mod pb {
    tonic::include_proto!("grpc.examples.echo");
}

mod layers;

use std::{
    error::Error,
    io::ErrorKind,
    net::ToSocketAddrs,
    pin::Pin,
    time::Duration,
    task::{ Context, Poll },
};
use log::info;
use tokio::{ sync::mpsc, runtime::Builder };
use tokio_stream::{ wrappers::ReceiverStream, Stream, StreamExt };
use tonic::{ Request, Response, Status, Streaming };
use tower::{ Layer, Service };
use pb::{ EchoRequest, EchoResponse };
type EchoResult<T> = Result<Response<T>, Status>;
type ResponseStream = Pin<Box<dyn Stream<Item = Result<EchoResponse, Status>> + Send>>;

fn match_for_io_error(err_status: &Status) -> Option<&std::io::Error> {
    let mut err: &(dyn Error + 'static) = err_status;

    loop {
        if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
            return Some(io_err);
        }

        // h2::Error do not expose std::io::Error with `source()`
        // https://github.com/hyperium/h2/pull/462
        if let Some(h2_err) = err.downcast_ref::<h2::Error>() {
            if let Some(io_err) = h2_err.get_io() {
                return Some(io_err);
            }
        }

        err = match err.source() {
            Some(err) => err,
            None => {
                return None;
            }
        };
    }
}

#[derive(Debug)]
pub struct EchoServer {}

#[tonic::async_trait]
impl pb::echo_server::Echo for EchoServer {
    async fn unary_echo(&self, _: Request<EchoRequest>) -> EchoResult<EchoResponse> {
        Err(Status::unimplemented("not implemented"))
    }

    type ServerStreamingEchoStream = ResponseStream;

    async fn server_streaming_echo(
        &self,
        req: Request<EchoRequest>
    ) -> EchoResult<Self::ServerStreamingEchoStream> {
        println!("EchoServer::server_streaming_echo");
        println!("\tclient connected from: {:?}", req.remote_addr());

        // creating infinite stream with requested message
        let repeat = std::iter::repeat(EchoResponse {
            message: req.into_inner().message,
        });
        let mut stream = Box::pin(tokio_stream::iter(repeat).throttle(Duration::from_millis(200)));

        // spawn and channel are required if you want handle "disconnect" functionality
        // the `out_stream` will not be polled after client disconnect
        let (tx, rx) = mpsc::channel(4);
        tokio::spawn(async move {
            while let Some(item) = stream.next().await {
                info!(
                    "Task completed on thread {:?} id : {:?}",
                    std::thread::current().name().unwrap(),
                    std::thread::current().id()
                );
                tokio::time::sleep(Duration::from_secs(2)).await;

                match tx.send(Result::<_, Status>::Ok(item)).await {
                    Ok(_) => {
                        // item (server response) was queued to be send to client
                    }
                    Err(_item) => {
                        // output_stream was build from rx and both are dropped
                        break;
                    }
                }
            }
            println!("\tclient disconnected");
        });

        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(output_stream) as Self::ServerStreamingEchoStream))
    }

    async fn client_streaming_echo(
        &self,
        _: Request<Streaming<EchoRequest>>
    ) -> EchoResult<EchoResponse> {
        Err(Status::unimplemented("not implemented"))
    }

    type BidirectionalStreamingEchoStream = ResponseStream;

    async fn bidirectional_streaming_echo(
        &self,
        req: Request<Streaming<EchoRequest>>
    ) -> EchoResult<Self::BidirectionalStreamingEchoStream> {
        println!("EchoServer::bidirectional_streaming_echo");

        let mut in_stream = req.into_inner();
        let (tx, rx) = mpsc::channel(128);

        // this spawn here is required if you want to handle connection error.
        // If we just map `in_stream` and write it back as `out_stream` the `out_stream`
        // will be drooped when connection error occurs and error will never be propagated
        // to mapped version of `in_stream`.
        tokio::spawn(async move {
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(v) =>
                        tx.send(Ok(EchoResponse { message: v.message })).await.expect("working rx"),
                    Err(err) => {
                        if let Some(io_err) = match_for_io_error(&err) {
                            if io_err.kind() == ErrorKind::BrokenPipe {
                                // here you can handle special case when client
                                // disconnected in unexpected way
                                eprintln!("\tclient disconnected: broken pipe");
                                break;
                            }
                        }

                        match tx.send(Err(err)).await {
                            Ok(_) => (),
                            Err(_err) => {
                                break;
                            } // response was droped
                        }
                    }
                }
            }
            println!("\tstream ended");
        });

        // echo just write the same data that was received
        let out_stream = ReceiverStream::new(rx);

        Ok(Response::new(Box::pin(out_stream) as Self::BidirectionalStreamingEchoStream))
    }
}

// #[tokio::main]
// #[tokio::main(flavor = "multi_thread", worker_threads = 4)]
// #[tokio::main(flavor = "current_thread")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let rt = Builder::new_multi_thread().worker_threads(1).enable_all().build().unwrap();
    let layer = tower::ServiceBuilder
        ::new()
        .layer(MyMiddlewareLayer::default())
        .layer(tonic::service::interceptor(layers::layer1))
        .layer(tonic::service::interceptor(layers::layer2))
        .into_inner();

    rt.block_on(async {
        let server = EchoServer {};

        let _ = tonic::transport::Server
            ::builder()
            .layer(layer)
            .add_service(pb::echo_server::EchoServer::new(server))
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

        // If cloning is really necessary, you could use let mut inner = mem::replace(&mut self.0, self.0.clone()) to "take" the ready service and replace it with the clone.

        let mut clone = self.inner.clone();
        // let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let response = clone.call(req).await?;
            Ok(response)
        })
    }
}
