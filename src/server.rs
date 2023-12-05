pub mod hello{
    tonic::include_proto!("hello");
}

use hello::service_server::{Service,ServiceServer};
use hello::{SampleReply,SampleRequest};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Default)]
pub  struct MyService{}

#[tonic::async_trait]
impl Service for MyService{
    async fn unary(&self,request:Request<SampleRequest>)-> Result<Response<SampleReply>, Status>{
            let response: SampleReply = SampleReply{message:format!("{:?}",request.remote_addr())};


            Ok(Response::new(response))
    }

    async fn server_stream(&self,request:Request<SampleRequest>){

    }

    

}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{
    let addr = "[::1]:50051".parse().unwrap();
    let service = ServiceServer::new(MyService::default());

    println!("GreeterServer listening on {}", addr);

    Server::builder()
        .add_service(service)
        .serve(addr)
        .await?;

    Ok(())
}