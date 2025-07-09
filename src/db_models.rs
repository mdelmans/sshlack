#[derive(Clone)]
pub struct User {
    pub username: String,
    pub authenticated: bool,
}

impl User {
    pub fn unauthenticated() -> Self {
        Self {
            username: String::new(),
            authenticated: false,
        }
    }

    pub fn authenticated(username: &str) -> Self {
        Self {
            username: username.to_string(),
            authenticated: true,
        }
    }
    pub fn bot() -> Self {
        Self {
            username: "*".to_string(),
            authenticated: true,
        }
    }
}

pub struct Message {
    pub content: String,
    pub sender: User,
}

impl Message {
    pub fn new(content: String, sender: User) -> Self {
        Self { content, sender }
    }
}
