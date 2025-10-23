use anyhow::{Context, Result};
use crate::AppState;
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

use super::repo::{FunctionInvocationRow, FunctionRow};
use nexus_types::{
    CreateFunctionReq, CreateFunctionResp, Function, FunctionInvocation, GetFunctionResp,
    InvokeFunctionReq, InvokeFunctionResp, ListFunctionsResp, ListInvocationsResp,
    UpdateFunctionReq,
};

// ========================================
// Function CRUD
// ========================================

pub async fn create_function(st: &AppState, req: CreateFunctionReq) -> Result<CreateFunctionResp> {
    // Validate runtime
    validate_runtime(&req.runtime)?;

    let id = Uuid::new_v4();
    let row = FunctionRow {
        id,
        name: req.name.clone(),
        runtime: req.runtime.clone(),
        code: req.code.clone(),
        handler: req.handler.clone(),
        timeout_seconds: req.timeout_seconds,
        memory_mb: req.memory_mb,
        vcpu: req.vcpu,
        env_vars: req.env_vars.clone(),
        vm_id: None,
        guest_ip: None,
        port: 3000,
        state: "creating".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        last_invoked_at: None,
    };

    super::repo::insert(&st.db, &row).await?;

    // Spawn dedicated MicroVM for this function in the background
    let st_clone = st.clone();
    let function_id = id;
    let function_name = req.name.clone();
    let runtime = req.runtime.clone();
    let code = req.code.clone();
    let handler = req.handler.clone();
    let vcpu = req.vcpu as u8;
    let memory_mb = req.memory_mb as u32;
    let env_vars = req.env_vars.clone();

    tokio::spawn(async move {
        match super::vm::create_function_vm(
            &st_clone,
            function_id,
            &function_name,
            &runtime,
            &code,
            &handler,
            vcpu,
            memory_mb,
            &env_vars,
        )
        .await
        {
            Ok(vm_id) => {
                eprintln!("[Function {}] VM created: {}", function_id, vm_id);

                // Update function with VM ID and state
                if let Err(e) = super::repo::update_vm_info(&st_clone.db, function_id, vm_id, None).await {
                    eprintln!("[Function {}] Failed to update VM info: {}", function_id, e);
                    let _ = super::repo::update_state(&st_clone.db, function_id, "error").await;
                    return;
                }
                let _ = super::repo::update_state(&st_clone.db, function_id, "booting").await;

                // Wait for guest IP to be available (up to 60 seconds)
                eprintln!("[Function {}] Waiting for VM guest IP...", function_id);
                let mut guest_ip: Option<String> = None;
                for attempt in 1..=60 {
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

                    // Check VM for guest IP
                    if let Ok(vm) = crate::features::vms::repo::get(&st_clone.db, vm_id).await {
                        if let Some(ip) = vm.guest_ip {
                            guest_ip = Some(ip);
                            break;
                        }
                    }

                    if attempt % 10 == 0 {
                        eprintln!("[Function {}] Still waiting for guest IP... ({}s)", function_id, attempt);
                    }
                }

                let guest_ip = match guest_ip {
                    Some(ip) => ip,
                    None => {
                        eprintln!("[Function {}] Timeout waiting for guest IP", function_id);
                        let _ = super::repo::update_state(&st_clone.db, function_id, "error").await;
                        return;
                    }
                };

                eprintln!("[Function {}] Got guest IP: {}", function_id, guest_ip);

                // Update function with guest IP and state
                let _ = super::repo::update_vm_info(&st_clone.db, function_id, vm_id, Some(&guest_ip)).await;
                let _ = super::repo::update_state(&st_clone.db, function_id, "deploying").await;

                // Inject function code via HTTP (will retry until successful)
                eprintln!("[Function {}] Injecting function code (will retry until runtime server is ready)...", function_id);
                match super::vm::update_function_code(&guest_ip, &runtime, &code, &handler).await {
                    Ok(_) => {
                        eprintln!("[Function {}] Code injection successful", function_id);
                        let _ = super::repo::update_state(&st_clone.db, function_id, "ready").await;
                    }
                    Err(e) => {
                        eprintln!("[Function {}] Code injection failed: {}", function_id, e);
                        let _ = super::repo::update_state(&st_clone.db, function_id, "error").await;
                    }
                }
            }
            Err(e) => {
                eprintln!("[Function {}] Failed to create VM: {}", function_id, e);
                let _ = super::repo::update_state(&st_clone.db, function_id, "error").await;
            }
        }
    });

    Ok(CreateFunctionResp { id })
}

pub async fn list_functions(db: &PgPool) -> Result<ListFunctionsResp> {
    let rows = super::repo::list(db).await?;
    let items = rows.into_iter().map(row_to_function).collect();
    Ok(ListFunctionsResp { items })
}

pub async fn get_function(db: &PgPool, id: Uuid) -> Result<GetFunctionResp> {
    let row = super::repo::get(db, id)
        .await?
        .context("Function not found")?;
    Ok(GetFunctionResp {
        item: row_to_function(row),
    })
}

pub async fn update_function(
    st: &AppState,
    id: Uuid,
    req: UpdateFunctionReq,
) -> Result<GetFunctionResp> {
    // Ensure function exists
    let existing = super::repo::get(&st.db, id)
        .await?
        .context("Function not found")?;

    // Validate runtime if provided
    if let Some(ref runtime) = req.runtime {
        validate_runtime(runtime)?;
    }

    // Update database
    super::repo::update(
        &st.db,
        id,
        req.name.as_deref(),
        req.runtime.as_deref(),
        req.code.as_deref(),
        req.handler.as_deref(),
        req.timeout_seconds,
        req.memory_mb,
        req.env_vars.as_ref(),
    )
    .await?;

    // If code or handler changed, reload it in the running VM
    let code_changed = req.code.is_some();
    let handler_changed = req.handler.is_some();

    if (code_changed || handler_changed) && existing.guest_ip.is_some() {
        let guest_ip = existing.guest_ip.unwrap();
        let new_code = req.code.unwrap_or(existing.code);
        let new_handler = req.handler.unwrap_or(existing.handler);
        let runtime = req.runtime.unwrap_or(existing.runtime);

        eprintln!("[Function {}] Code/handler updated, reloading in VM at {}", id, guest_ip);

        // Reload code in background (don't block the response)
        tokio::spawn(async move {
            if let Err(e) = super::vm::update_function_code(&guest_ip, &runtime, &new_code, &new_handler).await {
                eprintln!("[Function {}] Failed to reload code: {}", id, e);
            } else {
                eprintln!("[Function {}] Code reloaded successfully", id);
            }
        });
    }

    get_function(&st.db, id).await
}

pub async fn delete_function(st: &AppState, id: Uuid) -> Result<()> {
    // Get function to find VM ID
    let func = super::repo::get(&st.db, id)
        .await?
        .context("Function not found")?;

    // Delete the function's VM if it exists
    if let Some(vm_id) = func.vm_id {
        eprintln!("[Function {}] Deleting VM {}", id, vm_id);
        if let Err(e) = crate::features::vms::service::stop_and_delete(st, vm_id).await {
            eprintln!("[Function {}] Failed to delete VM: {}", id, e);
        }
    }

    // Delete function record
    super::repo::delete(&st.db, id).await?;
    Ok(())
}

// ========================================
// Function Invocation
// ========================================

pub async fn invoke_function(
    st: &AppState,
    id: Uuid,
    req: InvokeFunctionReq,
) -> Result<InvokeFunctionResp> {
    // Get function
    let func = super::repo::get(&st.db, id)
        .await?
        .context("Function not found")?;

    // Check if function is ready
    if func.state != "ready" {
        anyhow::bail!("Function is not ready (state: {})", func.state);
    }

    // Check if VM exists and has IP
    let guest_ip = func.guest_ip.as_ref().context("Function VM has no IP yet")?;

    // Generate request ID
    let request_id = Uuid::new_v4().to_string();

    // Invoke function via HTTP
    let start = Instant::now();
    let url = format!("http://{}:{}/invoke", guest_ip, func.port);

    eprintln!("[Function {}] Invoking at {}", id, url);

    let client = reqwest::Client::new();
    let http_result = client
        .post(&url)
        .json(&serde_json::json!({ "event": req.event }))
        .timeout(std::time::Duration::from_secs(func.timeout_seconds as u64 + 5))
        .send()
        .await;

    let duration_ms = start.elapsed().as_millis() as i64;

    let (status, response, logs, error) = match http_result {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(result) => {
                        let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        let response = result.get("response").cloned();
                        let logs = result.get("logs")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        let error = result.get("error").and_then(|v| v.as_str()).map(String::from);
                        (status, response, logs, error)
                    }
                    Err(e) => ("error".to_string(), None, vec![], Some(format!("Failed to parse response: {}", e))),
                }
            } else {
                ("error".to_string(), None, vec![], Some(format!("HTTP {}", resp.status())))
            }
        }
        Err(e) => {
            ("error".to_string(), None, vec![], Some(format!("HTTP request failed: {}", e)))
        }
    };

    // Store invocation
    let invocation_row = FunctionInvocationRow {
        id: Uuid::new_v4(),
        function_id: id,
        status: status.clone(),
        duration_ms,
        memory_used_mb: None,
        request_id: request_id.clone(),
        event: req.event,
        response: response.clone(),
        logs: logs.clone(),
        error: error.clone(),
        invoked_at: chrono::Utc::now(),
    };

    super::repo::insert_invocation(&st.db, &invocation_row).await?;

    // Update last invoked timestamp
    super::repo::update_last_invoked(&st.db, id).await?;

    Ok(InvokeFunctionResp {
        request_id,
        status,
        duration_ms,
        response,
        logs,
        error,
    })
}

pub async fn list_invocations(
    db: &PgPool,
    function_id: Uuid,
    status: Option<String>,
    limit: Option<i64>,
) -> Result<ListInvocationsResp> {
    let rows =
        super::repo::list_invocations(db, function_id, status.as_deref(), limit).await?;
    let items = rows.into_iter().map(invocation_row_to_type).collect();
    Ok(ListInvocationsResp { items })
}

// ========================================
// Helper Functions
// ========================================

fn row_to_function(row: FunctionRow) -> Function {
    Function {
        id: row.id,
        name: row.name,
        runtime: row.runtime,
        code: row.code,
        handler: row.handler,
        timeout_seconds: row.timeout_seconds,
        memory_mb: row.memory_mb,
        vcpu: row.vcpu,
        env_vars: row.env_vars,
        vm_id: row.vm_id,
        guest_ip: row.guest_ip,
        port: row.port,
        state: row.state,
        created_at: row.created_at,
        updated_at: row.updated_at,
        last_invoked_at: row.last_invoked_at,
    }
}

fn invocation_row_to_type(row: FunctionInvocationRow) -> FunctionInvocation {
    FunctionInvocation {
        id: row.id,
        function_id: row.function_id,
        status: row.status,
        duration_ms: row.duration_ms,
        memory_used_mb: row.memory_used_mb,
        request_id: row.request_id,
        event: row.event,
        response: row.response,
        logs: row.logs,
        error: row.error,
        invoked_at: row.invoked_at,
    }
}

fn validate_runtime(runtime: &str) -> Result<()> {
    match runtime {
        "node" | "python" | "go" | "rust" => Ok(()),
        _ => anyhow::bail!("Unsupported runtime: {}", runtime),
    }
}
