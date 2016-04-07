use iron::{BeforeMiddleware, IronError, IronResult, Request};

use authentication::Flow;

/// Handles Matrix's interactive authentication protocol for all API endpoints that require it.
pub struct InteractiveAuthentication {
    flows: Vec<Flow>,
}

impl InteractiveAuthentication {
    pub fn new(flows: Vec<Flow>) -> Self {
        InteractiveAuthentication {
            flows: flows,
        }
    }
}

impl BeforeMiddleware for InteractiveAuthentication {
    fn before(&self, _request: &mut Request) -> IronResult<()> {
        Ok(())
    }

    fn catch(&self, _request: &mut Request, err: IronError) -> IronResult<()> {
        Err(err)
    }
}
