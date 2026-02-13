use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};

use crate::{
    AppState,
    create_jwt,
    types::{SigninResponse, UserRequest, UserResponse},
};


pub async fn create_user(
    State(state):State<Arc<AppState>>,
    Json(body):Json<UserRequest>,
)->Result<Json<UserResponse>,(StatusCode,String)>{
    let user = state.db
       .create_user(&body.email,&body.password)
       .await
       .map_err(|e|(StatusCode::CONFLICT, e.to_string()))?;
    Ok(Json(UserResponse{
        id:user.id        
    }))
}


pub async fn signin(
    State(state): State<Arc<AppState>>,
    Json(body): Json<UserRequest>,
) -> Result<Json<SigninResponse>, (StatusCode, String)> {

    let user = state.db
        .get_user(&body.email)
        .await
        .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;

    if user.password != body.password {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Invalid email or password".to_string(),
        ));
    }

    let token = create_jwt(user.id);
    Ok(Json(
        SigninResponse{ 
            token
        }
    ))
}


