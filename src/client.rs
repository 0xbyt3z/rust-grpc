pub mod pb {
    tonic::include_proto!("grpc.examples.echo");
}

use std::{ time::Duration };
use tokio_stream::StreamExt;
use tonic::transport::Channel;
use log::info;

use pb::{ echo_client::EchoClient, EchoRequest };

async fn streaming_echo(mut client: EchoClient<Channel>, num: usize, msg: String) {
    let _ = tokio
        ::spawn(async move {
            let stream = client
                .server_streaming_echo(EchoRequest {
                    message: msg,
                }).await
                .unwrap()
                .into_inner();

            // stream is infinite - take just 5 elements and then disconnect
            let mut stream = stream.take(num);

            while let Some(item) = stream.next().await {
                info!(
                    "\treceived: {} on thread :{:?} id : {:?}",
                    item.unwrap().message,
                    std::thread::current().name().unwrap(),
                    std::thread::current().id()
                );
            }
            // stream is droped here and the disconnect info is send to server
        }).await
        .expect("Timeout");
}

#[tokio::main]
// #[tokio::main(flavor = "multi_thread", worker_threads = 10)]
// #[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let client = EchoClient::connect("http://[::1]:50051").await.unwrap();

    println!("Streaming echo:");
    streaming_echo(client, 10, std::env::args().nth(1).expect("Error please enter a string")).await;
    tokio::time::sleep(Duration::from_secs(1)).await; //do not mess server println functions

    Ok(())
}
