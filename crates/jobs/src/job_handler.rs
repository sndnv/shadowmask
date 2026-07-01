use std::future::Future;

use domain::job::Job;

use crate::error::JobError;

pub trait JobHandler {
    fn handle(&self, job: &Job) -> impl Future<Output = Result<(), JobError>> + Send;
}
