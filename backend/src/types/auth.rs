use serde::{Deserialize,Serialize};
use uuid::Uuid;

#[derive(Serialize,Deserialize)]
pub struct UserRequest{
    pub email : String,
    pub password : String
}

#[derive(Serialize,Deserialize)]
pub struct UserResponse{
    pub id : Uuid
}
#[derive(Serialize,Deserialize)]
pub struct SigninResponse{
    pub token: String
}
