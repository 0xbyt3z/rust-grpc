pub mod hello{
    tonic::include_proto!("hello");
}

use hello::SampleRequest;
use hello::service_client::ServiceClient;
use tonic::Request;

#[tokio::main]
async fn main() -> Result<(),Box<dyn std::error::Error>>{
    let mut channel = ServiceClient::connect("http://[::1]:50051").await?;

    let request = Request::new(SampleRequest{name:"hello".into()});

    let response = channel.unary(request).await?;

    println!("{:?}",response);
    Ok(())
    
}