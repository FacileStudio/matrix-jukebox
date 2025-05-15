use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Client {
    pub server_name: String,
    pub user_name: String,
    pub password: String,
}
