use crate::db_manager::DBManager;
use proto_bindings::job::job_runner_server::JobRunnerServer;
use server::MyJobRunner;
use tokio::signal::unix::{signal, SignalKind};
use tonic::transport::Server as TonicServer;

mod db_manager;
mod server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up two different ports for gRPC and HTTP
    let grpc_addr = "127.0.0.1:5050"
        .parse()
        .expect("Failed to parse gRPC address");

    // Build new DBManager that connects to the database
    let dbm = DBManager::new();
    // Connect to the database
    dbm.connect_to_db()
        .await
        .expect("Failed to connect to database");

    // gRPC server with DBManager
    let grpc_svc = JobRunnerServer::new(MyJobRunner::new(dbm));

    // Sigint signal handler that closes the DB connection upon shutdown
    let signal = grpc_sigint(dbm);

    // Build gRPC server with health service and signal sigint handler
    let grpc_server = TonicServer::builder()
        .add_service(grpc_svc)
        .serve_with_shutdown(grpc_addr, signal);

    // Create handler for each server
    //  https://github.com/hyperium/tonic/discussions/740
    let grpc_handle = tokio::spawn(grpc_server);

    println!("Started gRPC server on port {:?}", grpc_addr.port());
    // Join all servers together and start the the main loop
    let _ = tokio::try_join!(grpc_handle).expect("Failed to start gRPC and http server");

    Ok(())
}

async fn grpc_sigint(dbm: DBManager) {
    let _ = signal(SignalKind::terminate())
        .expect("failed to create a new SIGINT signal handler for gRPC")
        .recv()
        .await;

    // Shutdown the DB connection.
    dbm.close_db()
        .await
        .expect("Failed to close database connection");

    println!("gRPC shutdown complete");
}
