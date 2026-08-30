use chrono::NaiveDateTime;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct File {
    pub id: Uuid,
    pub user_id: Uuid,
    pub encrypted_filename: String,
    pub encrypted_file_key: String,
    pub encryption_nonce: String,
    pub encryption_version: String,
    pub file_size: i64,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
