use iron::{BeforeMiddleware, IronError, IronResult, Request};
use serde_json::from_value;

use authentication::{AuthParams, InteractiveAuth};
use error::APIError;
use middleware::JsonRequest;

/// Handles Matrix's interactive authentication protocol for all API endpoints that require it.
pub struct AuthRequest {
    interactive_auth: InteractiveAuth,
}

impl AuthRequest {
    pub fn new(interactive_auth: InteractiveAuth) -> Self {
        AuthRequest {
            interactive_auth: interactive_auth,
        }
    }

    pub fn interactive_auth(&self) -> &InteractiveAuth {
        &self.interactive_auth
    }
}

impl BeforeMiddleware for AuthRequest {
    fn before(&self, request: &mut Request) -> IronResult<()> {
        let interactive_auth = self.interactive_auth();

        let json = request.extensions.get::<JsonRequest>().expect(
           "middleware::JsonRequest should have ensured request body was JSON."
        );

        if let Some(auth_json) = json.find("auth") {
            if let Ok(ref auth_params) = from_value::<AuthParams>(auth_json.clone()) {
                if interactive_auth.validate(auth_params) {
                    return Ok(());
                } else {
                    return Err(
                        IronError::new(
                            APIError::unauthorized(),
                            interactive_auth,
                        )
                    );
                }
            }
        }

        Err(IronError::new(APIError::unauthorized(), interactive_auth))
    }

    fn catch(&self, _request: &mut Request, err: IronError) -> IronResult<()> {
        Err(err)
    }
}
