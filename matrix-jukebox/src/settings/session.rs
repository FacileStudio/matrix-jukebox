use matrix_sdk::authentication::matrix::MatrixSession;
use serde::{Deserialize, Serialize};

use super::db::Database;

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub user_session: MatrixSession,
    pub database: Database,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_token: Option<String>,
}
