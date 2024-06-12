use tonic::transport::Server;

use proto_bindings::proto::greeter_server::GreeterServer;

use crate::server::MyGreeter;

mod server;

// https://github.com/hyperium/tonic/blob/master/examples/src/helloworld/server.rs

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();
    let greeter = MyGreeter::default();

    println!("GreeterServer listening on {}", addr);

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
