use crate::features::users::repo::AuthenticatedUser;
use crate::AppState;
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    Extension, Json,
};
use nexus_types::{
    CreateFunctionReq, CreateFunctionResp, FunctionPathParams, GetFunctionResp, InvokeFunctionReq,
    InvokeFunctionResp, ListFunctionsResp, ListInvocationsParams, ListInvocationsResp, OkResponse,
    UpdateFunctionReq,
};

#[utoipa::path(
    post,
    path = "/v1/functions",
    request_body = CreateFunctionReq,
    responses(
        (status = 200, description = "Function created", body = CreateFunctionResp),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Failed to create function"),
    ),
    tag = "Functions"
)]
pub async fn create(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Json(req): Json<CreateFunctionReq>,
) -> Result<Json<CreateFunctionResp>, StatusCode> {
    let (user_id, username) = extract_user_info(user);
    let resp = super::service::create_function(&st, req, user_id, &username)
        .await
        .map_err(|e| {
            eprintln!("Failed to create function: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/v1/functions",
    responses(
        (status = 200, description = "Functions listed", body = ListFunctionsResp),
        (status = 500, description = "Failed to list functions"),
    ),
    tag = "Functions"
)]
pub async fn list(
    Extension(st): Extension<AppState>,
) -> Result<Json<ListFunctionsResp>, StatusCode> {
    let resp = super::service::list_functions(&st.db).await.map_err(|e| {
        eprintln!("Failed to list functions: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/v1/functions/{id}",
    params(FunctionPathParams),
    responses(
        (status = 200, description = "Function fetched", body = GetFunctionResp),
        (status = 404, description = "Function not found"),
        (status = 500, description = "Failed to fetch function"),
    ),
    tag = "Functions"
)]
pub async fn get(
    Extension(st): Extension<AppState>,
    Path(FunctionPathParams { id }): Path<FunctionPathParams>,
) -> Result<Json<GetFunctionResp>, StatusCode> {
    let resp = super::service::get_function(&st.db, id)
        .await
        .map_err(|e| {
            eprintln!("Failed to get function: {}", e);
            if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(resp))
}

#[utoipa::path(
    put,
    path = "/v1/functions/{id}",
    params(FunctionPathParams),
    request_body = UpdateFunctionReq,
    responses(
        (status = 200, description = "Function updated", body = GetFunctionResp),
        (status = 404, description = "Function not found"),
        (status = 500, description = "Failed to update function"),
    ),
    tag = "Functions"
)]
pub async fn update(
    Extension(st): Extension<AppState>,
    Path(FunctionPathParams { id }): Path<FunctionPathParams>,
    Json(req): Json<UpdateFunctionReq>,
) -> Result<Json<GetFunctionResp>, StatusCode> {
    let resp = super::service::update_function(&st, id, req)
        .await
        .map_err(|e| {
            eprintln!("Failed to update function: {}", e);
            if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(resp))
}

#[utoipa::path(
    delete,
    path = "/v1/functions/{id}",
    params(FunctionPathParams),
    responses(
        (status = 200, description = "Function deleted", body = OkResponse),
        (status = 404, description = "Function not found"),
        (status = 500, description = "Failed to delete function"),
    ),
    tag = "Functions"
)]
pub async fn delete(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(FunctionPathParams { id }): Path<FunctionPathParams>,
) -> Result<Json<OkResponse>, StatusCode> {
    let (user_id, username) = extract_user_info(user);
    super::service::delete_function(&st, id, user_id, &username)
        .await
        .map_err(|e| {
            eprintln!("Failed to delete function: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(OkResponse::default()))
}

#[utoipa::path(
    post,
    path = "/v1/functions/{id}/invoke",
    params(FunctionPathParams),
    request_body = InvokeFunctionReq,
    responses(
        (status = 200, description = "Function invoked", body = InvokeFunctionResp),
        (status = 404, description = "Function not found"),
        (status = 500, description = "Failed to invoke function"),
    ),
    tag = "Functions"
)]
pub async fn invoke(
    Extension(st): Extension<AppState>,
    user: Option<Extension<AuthenticatedUser>>,
    Path(FunctionPathParams { id }): Path<FunctionPathParams>,
    Json(req): Json<InvokeFunctionReq>,
) -> Result<Json<InvokeFunctionResp>, StatusCode> {
    let (user_id, username) = extract_user_info(user);
    let resp = super::service::invoke_function(&st, id, req, user_id, &username)
        .await
        .map_err(|e| {
            eprintln!("Failed to invoke function: {}", e);
            if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(resp))
}

#[utoipa::path(
    get,
    path = "/v1/functions/{id}/logs",
    params(FunctionPathParams, ListInvocationsParams),
    responses(
        (status = 200, description = "Invocation logs fetched", body = ListInvocationsResp),
        (status = 404, description = "Function not found"),
        (status = 500, description = "Failed to fetch logs"),
    ),
    tag = "Functions"
)]
pub async fn logs(
    Extension(st): Extension<AppState>,
    Path(FunctionPathParams { id }): Path<FunctionPathParams>,
    Query(params): Query<ListInvocationsParams>,
) -> Result<Json<ListInvocationsResp>, StatusCode> {
    let resp = super::service::list_invocations(&st.db, id, params.status, params.limit)
        .await
        .map_err(|e| {
            eprintln!("Failed to list invocations: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(resp))
}

fn extract_user_info(user: Option<Extension<AuthenticatedUser>>) -> (Option<uuid::Uuid>, String) {
    match user {
        Some(Extension(u)) => (Some(u.id), u.username),
        None => (None, "system".to_string()),
    }
}
