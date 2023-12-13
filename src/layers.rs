use tonic::{ Request, Response, Status, Streaming };

pub fn layer1(mut request: Request<()>) -> Result<Request<()>, Status> {
    if true {
        println!("layer1 called {:?}", request.metadata());

        // Append the metadata to the request
        request.metadata_mut().insert("key1", "value1".parse().unwrap());
        Ok(request)
    } else {
        Err(Status::not_found("Random Error"))
    }
}

pub fn layer2(request: Request<()>) -> Result<Request<()>, Status> {
    if true {
        println!("layer2 called {:?}", request.metadata());
        Ok(request)
    } else {
        Err(Status::not_found("Random Error"))
    }
}
