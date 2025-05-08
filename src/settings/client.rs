use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Client {
    pub homeserver_url: String,
    pub user_name: String,
    pub password: String,
}
