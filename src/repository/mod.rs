mod user;

use self::user::UserRepository;

pub struct Repository {
    pub users: UserRepository,
}

impl Repository {
    pub fn new() -> Self {
        Repository {
            users: UserRepository::new(),
        }
    }
}
