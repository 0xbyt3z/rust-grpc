// my_tower_service.rs

use tower::Service;
use tower::ServiceExt;

pub struct MyTowerService {
    // You can include any state or configuration needed for your service
    // For simplicity, we're not including any state in this example.
}

impl MyTowerService {
    pub fn new() -> Self {
        MyTowerService {}
    }
}

impl tower::Service<&'static str> for MyTowerService {
    type Response = String;
    type Error = String;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>
    >;

    pub fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>
    ) -> std::task::Poll<Result<(), Self::Error>> {
        // Simulate some asynchronous work, e.g., checking if the service is ready
        // For simplicity, we just return Ready(Ok(())) immediately.
        std::task::Poll::Ready(Ok(()))
    }

    pub fn call(&mut self, request: &'static str) -> Self::Future {
        // Simulate processing the request asynchronously
        let result = format!("Received: {}", request);

        // Wrap the result in a future
        Box::pin(async move { Ok(result) })
    }
}
