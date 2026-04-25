use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Client {
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub user_name: String,
    #[serde(default)]
    pub password: String,
}
