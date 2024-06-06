use crate::db_manager::DBManager;
use proto_bindings::job::job_runner_server::JobRunner;
use proto_bindings::job::{Empty, Job, JobList, JobReply, JobRequest};
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct MyJobRunner {
    db_manager: DBManager,
}

impl MyJobRunner {
    pub fn new(db_manager: DBManager) -> Self {
        Self { db_manager }
    }
}

#[tonic::async_trait]
impl JobRunner for MyJobRunner {
    async fn send_job(&self, request: Request<JobRequest>) -> Result<Response<JobReply>, Status> {
        println!("Got a request: {:?}", request);

        // Write into mock database
        self.db_manager
            .write_into_table()
            .await
            .expect("Failed to query database");

        let reply = JobReply {
            message: format!("Hello {}!", request.into_inner().name).into(),
        };

        Ok(Response::new(reply))
    }

    async fn list_jobs(&self, request: Request<Empty>) -> Result<Response<JobList>, Status> {
        println!("Got a request: {:?}", request);

        // Query mock database
        self.db_manager
            .query_table()
            .await
            .expect("Failed to query database");

        let reply = JobList {
            job: vec![Job {
                id: 1,
                name: "test".into(),
            }],
        };

        Ok(Response::new(reply))
    }
}
