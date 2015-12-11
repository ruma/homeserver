use iron::{BeforeMiddleware, IronError, IronResult, Request};

pub struct InteractiveAuthentication;

impl BeforeMiddleware for InteractiveAuthentication {
    fn before(&self, _request: &mut Request) -> IronResult<()> {
        Ok(())
    }

    fn catch(&self, _request: &mut Request, err: IronError) -> IronResult<()> {
        Err(err)
    }
}

