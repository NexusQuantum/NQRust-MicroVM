use crate::features::users::repo::AuthenticatedUser;
use crate::AppState;
use axum::{
    body::Body,
    extract::{Multipart, Path},
    http::{header, StatusCode},
    response::Response,
    Extension, Json,
};
use nexus_types::{
    ChangePasswordRequest, CreateUserRequest, GetPreferencesResponse, GetUserResponse,
    ListUsersResponse, LoginRequest, LoginResponse, UpdatePreferencesRequest, UpdateProfileRequest,
    UpdateUserRequest, User, UserPathParams,
};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};

#[utoipa::path(
    post,
    path = "/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials"),
        (status = 500, description = "Failed to authenticate"),
    ),
    tag = "Auth"
)]
pub async fn login(
    Extension(st): Extension<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let user = st
        .users
        .verify_password(&req.username, &req.password)
        .await
        .map_err(|e| {
            error!(?e, "failed to verify password");
            StatusCode::UNAUTHORIZED
        })?;

    let token = st.users.create_token(user.id, None).await.map_err(|e| {
        error!(?e, "failed to create token");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(LoginResponse {
        token,
        user: User {
            id: user.id,
            username: user.username.clone(),
            role: user.get_role(),
            avatar_path: user.avatar_path.clone(),
            timezone: user.timezone.clone(),
            theme: user.theme.clone(),
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        },
    }))
}

#[utoipa::path(
    get,
    path = "/v1/auth/me",
    responses(
        (status = 200, description = "Current user info", body = User),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to fetch user"),
    ),
    tag = "Auth"
)]
pub async fn me(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
) -> Result<Json<User>, StatusCode> {
    info!(user_id = ?user.id, "fetching current user info");

    let user_row = st.users.get_by_id(user.id).await.map_err(|e| {
        error!(?e, user_id = ?user.id, "failed to fetch user");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(user_id = ?user.id, "current user info fetched successfully");
    Ok(Json(User {
        id: user_row.id,
        username: user_row.username.clone(),
        role: user_row.get_role(),
        avatar_path: user_row.avatar_path.clone(),
        timezone: user_row.timezone.clone(),
        theme: user_row.theme.clone(),
        last_login_at: user_row.last_login_at,
        created_at: user_row.created_at,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/users",
    responses(
        (status = 200, description = "Users listed", body = ListUsersResponse),
        (status = 403, description = "Forbidden - admin only"),
        (status = 500, description = "Failed to list users"),
    ),
    tag = "Users"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<ListUsersResponse>, StatusCode> {
    let users = st.users.list().await.map_err(|e| {
        error!(?e, "failed to list users");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let items: Vec<User> = users
        .into_iter()
        .map(|u| User {
            id: u.id,
            username: u.username.clone(),
            role: u.get_role(),
            avatar_path: u.avatar_path.clone(),
            timezone: u.timezone.clone(),
            theme: u.theme.clone(),
            last_login_at: u.last_login_at,
            created_at: u.created_at,
        })
        .collect();

    Ok(Json(ListUsersResponse { items }))
}

#[utoipa::path(
    post,
    path = "/v1/users",
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "User created", body = User),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 500, description = "Failed to create user"),
    ),
    tag = "Users"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<User>, StatusCode> {
    let user = st
        .users
        .create_user(&req.username, &req.password, req.role)
        .await
        .map_err(|e| {
            error!(?e, "failed to create user");
            match e {
                crate::features::users::repo::UserRepoError::Sql(sqlx::Error::Database(db_err)) => {
                    if db_err.constraint().is_some() {
                        StatusCode::CONFLICT
                    } else {
                        StatusCode::INTERNAL_SERVER_ERROR
                    }
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

    Ok(Json(User {
        id: user.id,
        username: user.username.clone(),
        role: user.get_role(),
        avatar_path: user.avatar_path.clone(),
        timezone: user.timezone.clone(),
        theme: user.theme.clone(),
        last_login_at: user.last_login_at,
        created_at: user.created_at,
    }))
}

#[utoipa::path(
    get,
    path = "/v1/users/{id}",
    params(UserPathParams),
    responses(
        (status = 200, description = "User fetched", body = GetUserResponse),
        (status = 404, description = "User not found"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 500, description = "Failed to fetch user"),
    ),
    tag = "Users"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(UserPathParams { id }): Path<UserPathParams>,
) -> Result<Json<GetUserResponse>, StatusCode> {
    let user = st.users.get_by_id(id).await.map_err(|e| match e {
        crate::features::users::repo::UserRepoError::UserNotFound => StatusCode::NOT_FOUND,
        _ => {
            error!(?e, "failed to fetch user");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(GetUserResponse {
        item: User {
            id: user.id,
            username: user.username.clone(),
            role: user.get_role(),
            avatar_path: user.avatar_path.clone(),
            timezone: user.timezone.clone(),
            theme: user.theme.clone(),
            last_login_at: user.last_login_at,
            created_at: user.created_at,
        },
    }))
}

#[utoipa::path(
    patch,
    path = "/v1/users/{id}",
    params(UserPathParams),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = User),
        (status = 404, description = "User not found"),
        (status = 400, description = "Invalid request"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 500, description = "Failed to update user"),
    ),
    tag = "Users"
)]
pub async fn update(
    Extension(st): Extension<AppState>,
    Path(UserPathParams { id }): Path<UserPathParams>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<User>, StatusCode> {
    let user = st
        .users
        .update(
            id,
            req.username.as_deref(),
            req.password.as_deref(),
            req.role,
        )
        .await
        .map_err(|e| match e {
            crate::features::users::repo::UserRepoError::UserNotFound => StatusCode::NOT_FOUND,
            crate::features::users::repo::UserRepoError::InvalidRole(_) => StatusCode::BAD_REQUEST,
            _ => {
                error!(?e, "failed to update user");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(User {
        id: user.id,
        username: user.username.clone(),
        role: user.get_role(),
        avatar_path: user.avatar_path.clone(),
        timezone: user.timezone.clone(),
        theme: user.theme.clone(),
        last_login_at: user.last_login_at,
        created_at: user.created_at,
    }))
}

#[utoipa::path(
    delete,
    path = "/v1/users/{id}",
    params(UserPathParams),
    responses(
        (status = 200, description = "User deleted", body = nexus_types::OkResponse),
        (status = 404, description = "User not found"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 500, description = "Failed to delete user"),
    ),
    tag = "Users"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    Path(UserPathParams { id }): Path<UserPathParams>,
) -> Result<Json<nexus_types::OkResponse>, StatusCode> {
    st.users.delete(id).await.map_err(|e| match e {
        crate::features::users::repo::UserRepoError::UserNotFound => StatusCode::NOT_FOUND,
        _ => {
            error!(?e, "failed to delete user");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    Ok(Json(nexus_types::OkResponse::default()))
}

#[utoipa::path(
    get,
    path = "/v1/auth/me/preferences",
    responses(
        (status = 200, description = "Preferences retrieved", body = GetPreferencesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to fetch preferences"),
    ),
    tag = "Auth"
)]
pub async fn get_preferences(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
) -> Result<Json<GetPreferencesResponse>, StatusCode> {
    info!(user_id = ?user.id, "fetching preferences for user");

    let prefs = st.users.get_preferences(user.id).await.map_err(|e| {
        error!(?e, user_id = ?user.id, "failed to fetch preferences");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(user_id = ?user.id, "preferences fetched successfully");
    Ok(Json(GetPreferencesResponse { preferences: prefs }))
}

#[utoipa::path(
    patch,
    path = "/v1/auth/me/preferences",
    request_body = UpdatePreferencesRequest,
    responses(
        (status = 200, description = "Preferences updated", body = GetPreferencesResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to update preferences"),
    ),
    tag = "Auth"
)]
pub async fn update_preferences(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Result<Json<GetPreferencesResponse>, StatusCode> {
    info!(user_id = ?user.id, "updating preferences for user");

    let prefs = st
        .users
        .update_preferences(user.id, &req)
        .await
        .map_err(|e| {
            error!(?e, user_id = ?user.id, "failed to update preferences");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(user_id = ?user.id, "preferences updated successfully");
    Ok(Json(GetPreferencesResponse { preferences: prefs }))
}

#[utoipa::path(
    get,
    path = "/v1/auth/me/profile",
    responses(
        (status = 200, description = "Profile retrieved", body = User),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to fetch profile"),
    ),
    tag = "Auth"
)]
pub async fn get_profile(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
) -> Result<Json<User>, StatusCode> {
    info!(user_id = ?user.id, "fetching profile for user");

    let user_row = st.users.get_by_id(user.id).await.map_err(|e| {
        error!(?e, user_id = ?user.id, "failed to fetch user");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(user_id = ?user.id, "profile fetched successfully");
    Ok(Json(User {
        id: user_row.id,
        username: user_row.username.clone(),
        role: user_row.get_role(),
        last_login_at: user_row.last_login_at,
        avatar_path: user_row.avatar_path,
        timezone: user_row.timezone,
        theme: user_row.theme,
        created_at: user_row.created_at,
    }))
}

#[utoipa::path(
    patch,
    path = "/v1/auth/me/profile",
    request_body = UpdateProfileRequest,
    responses(
        (status = 200, description = "Profile updated", body = User),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to update profile"),
    ),
    tag = "Auth"
)]
pub async fn update_profile(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<User>, StatusCode> {
    info!(user_id = ?user.id, "updating profile for user");

    let user_row = st
        .users
        .update_profile(user.id, req.username.as_deref())
        .await
        .map_err(|e| {
            error!(?e, user_id = ?user.id, "failed to update profile");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(user_id = ?user.id, "profile updated successfully");
    Ok(Json(User {
        id: user_row.id,
        username: user_row.username.clone(),
        role: user_row.get_role(),
        last_login_at: user_row.last_login_at,
        avatar_path: user_row.avatar_path,
        timezone: user_row.timezone,
        theme: user_row.theme,
        created_at: user_row.created_at,
    }))
}

#[utoipa::path(
    post,
    path = "/v1/auth/me/password",
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed", body = nexus_types::OkResponse),
        (status = 401, description = "Invalid current password"),
        (status = 500, description = "Failed to change password"),
    ),
    tag = "Auth"
)]
pub async fn change_password(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<nexus_types::OkResponse>, StatusCode> {
    st.users
        .change_password(user.id, &req.current_password, &req.new_password)
        .await
        .map_err(|e| match e {
            crate::features::users::repo::UserRepoError::InvalidCredentials => {
                StatusCode::UNAUTHORIZED
            }
            _ => {
                error!(?e, "failed to change password");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;

    Ok(Json(nexus_types::OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/auth/me/avatar",
    responses(
        (status = 200, description = "Avatar uploaded", body = nexus_types::OkResponse),
        (status = 400, description = "Invalid file or size"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to upload avatar"),
    ),
    tag = "Auth"
)]
pub async fn upload_avatar(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
    mut multipart: Multipart,
) -> Result<Json<nexus_types::OkResponse>, StatusCode> {
    // Get file from multipart
    let mut file_data: Option<Vec<u8>> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("avatar") {
            let data = field.bytes().await.map_err(|e| {
                error!(?e, "failed to read multipart field");
                StatusCode::BAD_REQUEST
            })?;

            // Check file size (max 2MB)
            if data.len() > 2 * 1024 * 1024 {
                error!("avatar file too large: {} bytes", data.len());
                return Err(StatusCode::BAD_REQUEST);
            }

            file_data = Some(data.to_vec());
            break;
        }
    }

    let file_data = file_data.ok_or_else(|| {
        error!("no avatar file found in request");
        StatusCode::BAD_REQUEST
    })?;

    // Load and validate image
    let img = image::load_from_memory(&file_data).map_err(|e| {
        error!(?e, "failed to load image");
        StatusCode::BAD_REQUEST
    })?;

    // Resize to 500x500
    let resized = img.resize_exact(500, 500, image::imageops::FilterType::Lanczos3);

    // Create avatars directory if it doesn't exist
    let avatar_dir = PathBuf::from("/srv/images/avatars");
    fs::create_dir_all(&avatar_dir).await.map_err(|e| {
        error!(?e, "failed to create avatars directory");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Save as PNG
    let avatar_path = avatar_dir.join(format!("{}.png", user.id));
    let mut buffer = std::io::Cursor::new(Vec::new());
    resized
        .write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|e| {
            error!(?e, "failed to encode image");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Write to file
    let mut file = fs::File::create(&avatar_path).await.map_err(|e| {
        error!(?e, "failed to create avatar file");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    file.write_all(&buffer.into_inner()).await.map_err(|e| {
        error!(?e, "failed to write avatar file");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Update database
    st.users
        .set_avatar_path(user.id, avatar_path.to_str().unwrap())
        .await
        .map_err(|e| {
            error!(?e, "failed to update avatar path");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(user_id = ?user.id, "avatar uploaded successfully");

    Ok(Json(nexus_types::OkResponse::default()))
}

#[utoipa::path(
    get,
    path = "/v1/auth/me/avatar",
    responses(
        (status = 200, description = "Avatar image", content_type = "image/png"),
        (status = 404, description = "No avatar set"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to fetch avatar"),
    ),
    tag = "Auth"
)]
pub async fn get_my_avatar(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
) -> Result<Response, StatusCode> {
    let avatar_path = st.users.get_avatar_path(user.id).await.map_err(|e| {
        error!(?e, "failed to fetch avatar path");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let path = avatar_path.ok_or(StatusCode::NOT_FOUND)?;

    // Read file
    let data = fs::read(&path).await.map_err(|e| {
        error!(?e, path = %path, "failed to read avatar file");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .body(Body::from(data))
        .unwrap())
}

#[utoipa::path(
    get,
    path = "/v1/users/{id}/avatar",
    params(UserPathParams),
    responses(
        (status = 200, description = "Avatar image", content_type = "image/png"),
        (status = 404, description = "No avatar set"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 500, description = "Failed to fetch avatar"),
    ),
    tag = "Users"
)]
pub async fn get_user_avatar(
    Extension(st): Extension<AppState>,
    Path(UserPathParams { id }): Path<UserPathParams>,
) -> Result<Response, StatusCode> {
    let avatar_path = st.users.get_avatar_path(id).await.map_err(|e| {
        error!(?e, "failed to fetch avatar path");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let path = avatar_path.ok_or(StatusCode::NOT_FOUND)?;

    // Read file
    let data = fs::read(&path).await.map_err(|e| {
        error!(?e, path = %path, "failed to read avatar file");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .body(Body::from(data))
        .unwrap())
}

#[utoipa::path(
    delete,
    path = "/v1/auth/me/avatar",
    responses(
        (status = 200, description = "Avatar deleted", body = nexus_types::OkResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Failed to delete avatar"),
    ),
    tag = "Auth"
)]
pub async fn delete_avatar(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
) -> Result<Json<nexus_types::OkResponse>, StatusCode> {
    // Get current avatar path to delete file
    if let Ok(Some(path)) = st.users.get_avatar_path(user.id).await {
        // Delete file (ignore errors if file doesn't exist)
        let _ = fs::remove_file(&path).await;
    }

    // Update database
    st.users.delete_avatar(user.id).await.map_err(|e| {
        error!(?e, "failed to delete avatar");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(user_id = ?user.id, "avatar deleted successfully");

    Ok(Json(nexus_types::OkResponse::default()))
}
