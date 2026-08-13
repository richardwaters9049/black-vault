use uuid::Uuid;


#[derive(Debug, Clone)]
pub struct UserCredentials {
    pub id: Uuid,
    pub email: String,
    #[allow(dead_code)]
    pub password_hash: String,
}