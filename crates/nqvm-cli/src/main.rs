mod client;
mod config;
mod output;
mod shell;

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use client::Client;
use config::{default_config_path, Config};
use nexus_types::{LoginRequest, LoginResponse};
use output::{print_value, read_body_file, OutputMode};
use serde_json::{json, Map, Value};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "nqvm")]
#[command(about = "Command-line client for NQR-MicroVM")]
struct Cli {
    #[arg(long, global = true)]
    api_url: Option<String>,
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true, value_enum)]
    output: Option<OutputMode>,
    #[arg(long, global = true)]
    token: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Login(LoginArgs),
    Logout(ConfirmArgs),
    #[command(subcommand)]
    Auth(AuthCommand),
    #[command(subcommand)]
    Vm(VmCommand),
    #[command(subcommand)]
    Container(ContainerCommand),
    #[command(subcommand)]
    Function(FunctionCommand),
    #[command(subcommand)]
    Template(TemplateCommand),
    #[command(subcommand)]
    Host(HostCommand),
    #[command(subcommand)]
    Image(ImageCommand),
    #[command(subcommand)]
    Snapshot(SnapshotCommand),
    #[command(subcommand)]
    Volume(VolumeCommand),
    #[command(subcommand)]
    Network(NetworkCommand),
    #[command(name = "storage-backend")]
    #[command(subcommand)]
    StorageBackend(StorageBackendCommand),
    #[command(subcommand)]
    User(UserCommand),
    #[command(subcommand)]
    License(LicenseCommand),
}

#[derive(Debug, Args)]
struct LoginArgs {
    #[arg(long)]
    api_url: Option<String>,
    #[arg(long)]
    username: String,
    #[arg(long)]
    password: Option<String>,
}

#[derive(Debug, Subcommand)]
enum AuthCommand {
    Status,
}

#[derive(Debug, Args)]
struct ConfirmArgs {
    #[arg(long)]
    yes: bool,
}

#[derive(Debug, Args)]
struct IdArgs {
    id: Uuid,
}

#[derive(Debug, Args)]
struct IdConfirmArgs {
    id: Uuid,
    #[arg(long)]
    yes: bool,
}

#[derive(Debug, Args)]
struct FileArgs {
    #[arg(long)]
    file: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum VmCommand {
    List,
    Get(IdArgs),
    Create(VmCreateArgs),
    Update(VmUpdateArgs),
    Start(IdArgs),
    Stop(IdArgs),
    Pause(IdArgs),
    Resume(IdArgs),
    Delete(IdConfirmArgs),
    Shell(VmShellArgs),
}

#[derive(Debug, Args)]
struct VmShellArgs {
    id: Uuid,
    #[arg(long)]
    no_credentials: bool,
}

#[derive(Debug, Args)]
struct VmCreateArgs {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    vcpu: Option<u8>,
    #[arg(long)]
    mem_mib: Option<u32>,
    #[arg(long)]
    kernel_image_id: Option<Uuid>,
    #[arg(long)]
    rootfs_image_id: Option<Uuid>,
    #[arg(long)]
    kernel_path: Option<String>,
    #[arg(long)]
    rootfs_path: Option<String>,
    #[arg(long)]
    rootfs_size_mb: Option<u32>,
    #[arg(long)]
    network_id: Option<Uuid>,
    #[arg(long)]
    backend_id: Option<Uuid>,
    #[arg(long)]
    username: Option<String>,
    #[arg(long)]
    password: Option<String>,
    #[arg(long, value_delimiter = ',')]
    tags: Vec<String>,
}

#[derive(Debug, Args)]
struct VmUpdateArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, value_delimiter = ',')]
    tags: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum ContainerCommand {
    List,
    Get(IdArgs),
    Deploy(ContainerDeployArgs),
    Update(ContainerUpdateArgs),
    Start(IdArgs),
    Stop(IdArgs),
    Restart(IdArgs),
    Pause(IdArgs),
    Resume(IdArgs),
    Delete(IdConfirmArgs),
}

#[derive(Debug, Args)]
struct ContainerDeployArgs {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    image: Option<String>,
    #[arg(long)]
    command: Option<String>,
    #[arg(long)]
    cpu_limit: Option<f32>,
    #[arg(long)]
    memory_limit_mb: Option<i32>,
    #[arg(long)]
    restart_policy: Option<String>,
}

#[derive(Debug, Args)]
struct ContainerUpdateArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    cpu_limit: Option<f32>,
    #[arg(long)]
    memory_limit_mb: Option<i32>,
    #[arg(long)]
    restart_policy: Option<String>,
}

#[derive(Debug, Subcommand)]
enum FunctionCommand {
    List,
    Get(IdArgs),
    Create(FunctionCreateArgs),
    Update(FunctionUpdateArgs),
    Invoke(FunctionInvokeArgs),
    Logs(FunctionLogsArgs),
    Delete(IdConfirmArgs),
}

#[derive(Debug, Args)]
struct FunctionCreateArgs {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    runtime: Option<String>,
    #[arg(long)]
    code: Option<String>,
    #[arg(long)]
    code_file: Option<PathBuf>,
    #[arg(long)]
    handler: Option<String>,
    #[arg(long)]
    timeout_seconds: Option<i32>,
    #[arg(long)]
    memory_mb: Option<i32>,
    #[arg(long)]
    vcpu: Option<i32>,
}

#[derive(Debug, Args)]
struct FunctionUpdateArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    runtime: Option<String>,
    #[arg(long)]
    code: Option<String>,
    #[arg(long)]
    code_file: Option<PathBuf>,
    #[arg(long)]
    handler: Option<String>,
    #[arg(long)]
    timeout_seconds: Option<i32>,
    #[arg(long)]
    memory_mb: Option<i32>,
}

#[derive(Debug, Args)]
struct FunctionInvokeArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    event: Option<String>,
}

#[derive(Debug, Args)]
struct FunctionLogsArgs {
    id: Uuid,
    #[arg(long)]
    status: Option<String>,
    #[arg(long)]
    limit: Option<i64>,
}

#[derive(Debug, Subcommand)]
enum TemplateCommand {
    List,
    Get(IdArgs),
    Create(FileArgs),
    Update(TemplateUpdateArgs),
    Instantiate(TemplateInstantiateArgs),
    Delete(IdConfirmArgs),
}

#[derive(Debug, Args)]
struct TemplateUpdateArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct TemplateInstantiateArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Subcommand)]
enum HostCommand {
    List,
    Get(IdArgs),
    Delete(IdConfirmArgs),
}

#[derive(Debug, Subcommand)]
enum ImageCommand {
    List(ImageListArgs),
    Get(IdArgs),
    Create(ImageCreateArgs),
    Delete(IdConfirmArgs),
    DockerhubSearch(DockerhubSearchArgs),
    DockerhubTags(DockerhubTagsArgs),
    DockerhubDownload(DockerhubDownloadArgs),
    DockerhubProgress(DockerhubProgressArgs),
    Preload,
}

#[derive(Debug, Args)]
struct ImageListArgs {
    #[arg(long)]
    kind: Option<String>,
    #[arg(long)]
    project: Option<String>,
    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Args)]
struct ImageCreateArgs {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    kind: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    host_path: Option<String>,
    #[arg(long)]
    sha256: Option<String>,
    #[arg(long)]
    size: Option<i64>,
    #[arg(long)]
    project: Option<String>,
}

#[derive(Debug, Args)]
struct DockerhubSearchArgs {
    query: String,
    #[arg(long)]
    limit: Option<i32>,
}

#[derive(Debug, Args)]
struct DockerhubTagsArgs {
    image: String,
}

#[derive(Debug, Args)]
struct DockerhubDownloadArgs {
    image: String,
}

#[derive(Debug, Args)]
struct DockerhubProgressArgs {
    image: String,
}

#[derive(Debug, Subcommand)]
enum SnapshotCommand {
    List(SnapshotListArgs),
    Get(IdArgs),
    Create(SnapshotCreateArgs),
    Instantiate(SnapshotInstantiateArgs),
    Delete(IdConfirmArgs),
}

#[derive(Debug, Args)]
struct SnapshotListArgs {
    #[arg(long)]
    vm_id: Uuid,
}

#[derive(Debug, Args)]
struct SnapshotCreateArgs {
    #[arg(long)]
    vm_id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    snapshot_type: Option<String>,
    #[arg(long)]
    track_dirty_pages: Option<bool>,
}

#[derive(Debug, Args)]
struct SnapshotInstantiateArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
}

#[derive(Debug, Subcommand)]
enum VolumeCommand {
    List,
    Get(IdArgs),
    Create(VolumeCreateArgs),
    Attach(VolumeAttachArgs),
    Detach(VolumeDetachArgs),
    Delete(IdConfirmArgs),
}

#[derive(Debug, Args)]
struct VolumeCreateArgs {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    size_gb: Option<i64>,
    #[arg(long = "type")]
    volume_type: Option<String>,
    #[arg(long)]
    host_id: Option<Uuid>,
    #[arg(long)]
    backend_id: Option<Uuid>,
}

#[derive(Debug, Args)]
struct VolumeAttachArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    vm_id: Option<Uuid>,
    #[arg(long)]
    drive_id: Option<String>,
}

#[derive(Debug, Args)]
struct VolumeDetachArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    vm_id: Option<Uuid>,
}

#[derive(Debug, Subcommand)]
enum NetworkCommand {
    List,
    Get(IdArgs),
    Create(NetworkCreateArgs),
    Update(NetworkUpdateArgs),
    Delete(IdConfirmArgs),
    Suggest(HostQueryArgs),
    Interfaces(HostQueryArgs),
    Retry(IdArgs),
    Vms(IdArgs),
}

#[derive(Debug, Args)]
struct NetworkCreateArgs {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long = "type")]
    network_type: Option<String>,
    #[arg(long)]
    host_id: Option<Uuid>,
    #[arg(long)]
    cidr: Option<String>,
    #[arg(long)]
    vlan_id: Option<i32>,
    #[arg(long)]
    dhcp_enabled: Option<bool>,
    #[arg(long)]
    uplink_interface: Option<String>,
    #[arg(long)]
    gateway_host_id: Option<Uuid>,
}

#[derive(Debug, Args)]
struct NetworkUpdateArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    description: Option<String>,
    #[arg(long)]
    cidr: Option<String>,
    #[arg(long)]
    gateway: Option<String>,
}

#[derive(Debug, Args)]
struct HostQueryArgs {
    #[arg(long)]
    host_id: Uuid,
}

#[derive(Debug, Subcommand)]
enum StorageBackendCommand {
    List,
    Get(IdArgs),
}

#[derive(Debug, Subcommand)]
enum UserCommand {
    List,
    Get(IdArgs),
    Create(UserCreateArgs),
    Update(UserUpdateArgs),
    Delete(IdConfirmArgs),
}

#[derive(Debug, Args)]
struct UserCreateArgs {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    username: Option<String>,
    #[arg(long)]
    password: Option<String>,
    #[arg(long)]
    role: Option<String>,
}

#[derive(Debug, Args)]
struct UserUpdateArgs {
    id: Uuid,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    username: Option<String>,
    #[arg(long)]
    password: Option<String>,
    #[arg(long)]
    role: Option<String>,
}

#[derive(Debug, Subcommand)]
enum LicenseCommand {
    Status,
    Activate(LicenseActivateArgs),
    EulaStatus,
    AcceptEula(EulaAcceptArgs),
}

#[derive(Debug, Args)]
struct LicenseActivateArgs {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    license_key: Option<String>,
}

#[derive(Debug, Args)]
struct EulaAcceptArgs {
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    version: Option<String>,
    #[arg(long, default_value = "en")]
    language: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config.clone().unwrap_or_else(default_config_path);
    let mut cfg = Config::load(&config_path)?;
    let output = cli
        .output
        .unwrap_or_else(|| OutputMode::from_config(&cfg.default_output));

    if let Some(api_url) = &cli.api_url {
        cfg.api_url = api_url.clone();
    }

    match cli.command {
        Command::Login(args) => login(args, cfg, &config_path, output).await,
        Command::Logout(args) => {
            confirm(args.yes, "Remove stored nqvm token?")?;
            cfg.token = None;
            cfg.save(&config_path)?;
            println!("Logged out");
            Ok(())
        }
        command => {
            let token = cli.token.or_else(|| cfg.token.clone());
            let client = Client::new(cfg.api_url, token);
            if let Some(value) = run_command(&client, command).await? {
                print_value(&value, output)?;
            }
            Ok(())
        }
    }
}

async fn login(
    args: LoginArgs,
    mut cfg: Config,
    config_path: &std::path::Path,
    output: OutputMode,
) -> Result<()> {
    if let Some(api_url) = args.api_url {
        cfg.api_url = api_url;
    }

    let password = match args.password {
        Some(password) => password,
        None => rpassword::prompt_password("Password: ")?,
    };

    let client = Client::new(cfg.api_url.clone(), None);
    let response: LoginResponse = client
        .post(
            "/v1/auth/login",
            &LoginRequest {
                username: args.username,
                password,
            },
        )
        .await?;

    cfg.token = Some(response.token);
    cfg.save(config_path)?;
    print_value(&serde_json::to_value(response.user)?, output)
}

async fn run_command(client: &Client, command: Command) -> Result<Option<Value>> {
    match command {
        Command::Auth(AuthCommand::Status) => client.get("/v1/auth/me").await.map(Some),
        Command::Vm(cmd) => run_vm(client, cmd).await,
        Command::Container(cmd) => run_container(client, cmd).await.map(Some),
        Command::Function(cmd) => run_function(client, cmd).await.map(Some),
        Command::Template(cmd) => run_template(client, cmd).await.map(Some),
        Command::Host(cmd) => run_host(client, cmd).await.map(Some),
        Command::Image(cmd) => run_image(client, cmd).await.map(Some),
        Command::Snapshot(cmd) => run_snapshot(client, cmd).await.map(Some),
        Command::Volume(cmd) => run_volume(client, cmd).await.map(Some),
        Command::Network(cmd) => run_network(client, cmd).await.map(Some),
        Command::StorageBackend(cmd) => run_storage_backend(client, cmd).await.map(Some),
        Command::User(cmd) => run_user(client, cmd).await.map(Some),
        Command::License(cmd) => run_license(client, cmd).await.map(Some),
        Command::Login(_) | Command::Logout(_) => unreachable!("handled before client dispatch"),
    }
}

async fn run_vm(client: &Client, cmd: VmCommand) -> Result<Option<Value>> {
    match cmd {
        VmCommand::List => client.get("/v1/vms").await.map(Some),
        VmCommand::Get(args) => client.get(&format!("/v1/vms/{}", args.id)).await.map(Some),
        VmCommand::Create(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "vcpu", args.vcpu)?;
            set(&mut body, "mem_mib", args.mem_mib)?;
            set(&mut body, "kernel_image_id", args.kernel_image_id)?;
            set(&mut body, "rootfs_image_id", args.rootfs_image_id)?;
            set(&mut body, "kernel_path", args.kernel_path)?;
            set(&mut body, "rootfs_path", args.rootfs_path)?;
            set(&mut body, "rootfs_size_mb", args.rootfs_size_mb)?;
            set(&mut body, "network_id", args.network_id)?;
            set(&mut body, "backend_id", args.backend_id)?;
            set(&mut body, "username", args.username)?;
            set(&mut body, "password", args.password)?;
            if !args.tags.is_empty() {
                set(&mut body, "tags", Some(args.tags))?;
            }
            client.post("/v1/vms", &body).await.map(Some)
        }
        VmCommand::Update(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            if !args.tags.is_empty() {
                set(&mut body, "tags", Some(args.tags))?;
            }
            client
                .patch(&format!("/v1/vms/{}", args.id), &body)
                .await
                .map(Some)
        }
        VmCommand::Start(args) => client
            .post(&format!("/v1/vms/{}/start", args.id), &json!({}))
            .await
            .map(Some),
        VmCommand::Stop(args) => client
            .post(&format!("/v1/vms/{}/stop", args.id), &json!({}))
            .await
            .map(Some),
        VmCommand::Pause(args) => client
            .post(&format!("/v1/vms/{}/pause", args.id), &json!({}))
            .await
            .map(Some),
        VmCommand::Resume(args) => client
            .post(&format!("/v1/vms/{}/resume", args.id), &json!({}))
            .await
            .map(Some),
        VmCommand::Delete(args) => {
            confirm(args.yes, "Delete VM?")?;
            client
                .delete(&format!("/v1/vms/{}", args.id))
                .await
                .map(Some)
        }
        VmCommand::Shell(args) => {
            shell::connect_vm_shell(client, args.id, !args.no_credentials).await?;
            Ok(None)
        }
    }
}

async fn run_container(client: &Client, cmd: ContainerCommand) -> Result<Value> {
    match cmd {
        ContainerCommand::List => client.get("/v1/containers").await,
        ContainerCommand::Get(args) => client.get(&format!("/v1/containers/{}", args.id)).await,
        ContainerCommand::Deploy(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "image", args.image)?;
            set(&mut body, "command", args.command)?;
            set(&mut body, "cpu_limit", args.cpu_limit)?;
            set(&mut body, "memory_limit_mb", args.memory_limit_mb)?;
            set(&mut body, "restart_policy", args.restart_policy)?;
            client.post("/v1/containers", &body).await
        }
        ContainerCommand::Update(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "cpu_limit", args.cpu_limit)?;
            set(&mut body, "memory_limit_mb", args.memory_limit_mb)?;
            set(&mut body, "restart_policy", args.restart_policy)?;
            client
                .patch(&format!("/v1/containers/{}", args.id), &body)
                .await
        }
        ContainerCommand::Start(args) => {
            client
                .post(&format!("/v1/containers/{}/start", args.id), &json!({}))
                .await
        }
        ContainerCommand::Stop(args) => {
            client
                .post(&format!("/v1/containers/{}/stop", args.id), &json!({}))
                .await
        }
        ContainerCommand::Restart(args) => {
            client
                .post(&format!("/v1/containers/{}/restart", args.id), &json!({}))
                .await
        }
        ContainerCommand::Pause(args) => {
            client
                .post(&format!("/v1/containers/{}/pause", args.id), &json!({}))
                .await
        }
        ContainerCommand::Resume(args) => {
            client
                .post(&format!("/v1/containers/{}/resume", args.id), &json!({}))
                .await
        }
        ContainerCommand::Delete(args) => {
            confirm(args.yes, "Delete container?")?;
            client.delete(&format!("/v1/containers/{}", args.id)).await
        }
    }
}

async fn run_function(client: &Client, cmd: FunctionCommand) -> Result<Value> {
    match cmd {
        FunctionCommand::List => client.get("/v1/functions").await,
        FunctionCommand::Get(args) => client.get(&format!("/v1/functions/{}", args.id)).await,
        FunctionCommand::Create(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "runtime", args.runtime)?;
            set(
                &mut body,
                "code",
                args.code.or(read_optional_text(args.code_file)?),
            )?;
            set(&mut body, "handler", args.handler)?;
            set(&mut body, "timeout_seconds", args.timeout_seconds)?;
            set(&mut body, "memory_mb", args.memory_mb)?;
            set(&mut body, "vcpu", args.vcpu)?;
            client.post("/v1/functions", &body).await
        }
        FunctionCommand::Update(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "runtime", args.runtime)?;
            set(
                &mut body,
                "code",
                args.code.or(read_optional_text(args.code_file)?),
            )?;
            set(&mut body, "handler", args.handler)?;
            set(&mut body, "timeout_seconds", args.timeout_seconds)?;
            set(&mut body, "memory_mb", args.memory_mb)?;
            client
                .put(&format!("/v1/functions/{}", args.id), &body)
                .await
        }
        FunctionCommand::Invoke(args) => {
            let mut body = body_from_file(args.file)?;
            if let Some(event) = args.event {
                set_value(
                    &mut body,
                    "event",
                    serde_json::from_str(&event).unwrap_or(Value::String(event)),
                )?;
            }
            client
                .post(&format!("/v1/functions/{}/invoke", args.id), &body)
                .await
        }
        FunctionCommand::Logs(args) => {
            client
                .get(&query_path(
                    &format!("/v1/functions/{}/logs", args.id),
                    &[
                        ("status", args.status),
                        ("limit", args.limit.map(|v| v.to_string())),
                    ],
                ))
                .await
        }
        FunctionCommand::Delete(args) => {
            confirm(args.yes, "Delete function?")?;
            client.delete(&format!("/v1/functions/{}", args.id)).await
        }
    }
}

async fn run_template(client: &Client, cmd: TemplateCommand) -> Result<Value> {
    match cmd {
        TemplateCommand::List => client.get("/v1/templates").await,
        TemplateCommand::Get(args) => client.get(&format!("/v1/templates/{}", args.id)).await,
        TemplateCommand::Create(args) => {
            client
                .post("/v1/templates", &required_body(args.file)?)
                .await
        }
        TemplateCommand::Update(args) => {
            client
                .patch(
                    &format!("/v1/templates/{}", args.id),
                    &required_body(args.file)?,
                )
                .await
        }
        TemplateCommand::Instantiate(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            client
                .post(&format!("/v1/templates/{}/instantiate", args.id), &body)
                .await
        }
        TemplateCommand::Delete(args) => {
            confirm(args.yes, "Delete template?")?;
            client.delete(&format!("/v1/templates/{}", args.id)).await
        }
    }
}

async fn run_host(client: &Client, cmd: HostCommand) -> Result<Value> {
    match cmd {
        HostCommand::List => client.get("/v1/hosts").await,
        HostCommand::Get(args) => client.get(&format!("/v1/hosts/{}", args.id)).await,
        HostCommand::Delete(args) => {
            confirm(args.yes, "Delete host?")?;
            client.delete(&format!("/v1/hosts/{}", args.id)).await
        }
    }
}

async fn run_image(client: &Client, cmd: ImageCommand) -> Result<Value> {
    match cmd {
        ImageCommand::List(args) => {
            client
                .get(&query_path(
                    "/v1/images",
                    &[
                        ("kind", args.kind),
                        ("project", args.project),
                        ("name", args.name),
                    ],
                ))
                .await
        }
        ImageCommand::Get(args) => client.get(&format!("/v1/images/{}", args.id)).await,
        ImageCommand::Create(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "kind", args.kind)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "host_path", args.host_path)?;
            set(&mut body, "sha256", args.sha256)?;
            set(&mut body, "size", args.size)?;
            set(&mut body, "project", args.project)?;
            client.post("/v1/images", &body).await
        }
        ImageCommand::Delete(args) => {
            confirm(args.yes, "Delete image?")?;
            client.delete(&format!("/v1/images/{}", args.id)).await
        }
        ImageCommand::DockerhubSearch(args) => {
            client
                .post(
                    "/v1/images/dockerhub/search",
                    &json!({"query": args.query, "limit": args.limit}),
                )
                .await
        }
        ImageCommand::DockerhubTags(args) => {
            client
                .post("/v1/images/dockerhub/tags", &json!({"image": args.image}))
                .await
        }
        ImageCommand::DockerhubDownload(args) => {
            client
                .post(
                    "/v1/images/dockerhub/download",
                    &json!({"image": args.image}),
                )
                .await
        }
        ImageCommand::DockerhubProgress(args) => {
            client
                .get(&format!(
                    "/v1/images/dockerhub/download/progress/{}",
                    urlencoding::encode(&args.image)
                ))
                .await
        }
        ImageCommand::Preload => {
            client
                .post("/v1/images/dockerhub/preload", &json!({}))
                .await
        }
    }
}

async fn run_snapshot(client: &Client, cmd: SnapshotCommand) -> Result<Value> {
    match cmd {
        SnapshotCommand::List(args) => {
            client
                .get(&format!("/v1/vms/{}/snapshots", args.vm_id))
                .await
        }
        SnapshotCommand::Get(args) => client.get(&format!("/v1/snapshots/{}", args.id)).await,
        SnapshotCommand::Create(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "snapshot_type", args.snapshot_type)?;
            set(&mut body, "track_dirty_pages", args.track_dirty_pages)?;
            client
                .post(&format!("/v1/vms/{}/snapshots", args.vm_id), &body)
                .await
        }
        SnapshotCommand::Instantiate(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            client
                .post(&format!("/v1/snapshots/{}/instantiate", args.id), &body)
                .await
        }
        SnapshotCommand::Delete(args) => {
            confirm(args.yes, "Delete snapshot?")?;
            client.delete(&format!("/v1/snapshots/{}", args.id)).await
        }
    }
}

async fn run_volume(client: &Client, cmd: VolumeCommand) -> Result<Value> {
    match cmd {
        VolumeCommand::List => client.get("/v1/volumes").await,
        VolumeCommand::Get(args) => client.get(&format!("/v1/volumes/{}", args.id)).await,
        VolumeCommand::Create(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "description", args.description)?;
            set(&mut body, "size_gb", args.size_gb)?;
            set(&mut body, "type", args.volume_type)?;
            set(&mut body, "host_id", args.host_id)?;
            set(&mut body, "backend_id", args.backend_id)?;
            client.post("/v1/volumes", &body).await
        }
        VolumeCommand::Attach(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "vm_id", args.vm_id)?;
            set(&mut body, "drive_id", args.drive_id)?;
            client
                .post(&format!("/v1/volumes/{}/attach", args.id), &body)
                .await
        }
        VolumeCommand::Detach(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "vm_id", args.vm_id)?;
            client
                .post(&format!("/v1/volumes/{}/detach", args.id), &body)
                .await
        }
        VolumeCommand::Delete(args) => {
            confirm(args.yes, "Delete volume?")?;
            client.delete(&format!("/v1/volumes/{}", args.id)).await
        }
    }
}

async fn run_network(client: &Client, cmd: NetworkCommand) -> Result<Value> {
    match cmd {
        NetworkCommand::List => client.get("/v1/networks").await,
        NetworkCommand::Get(args) => client.get(&format!("/v1/networks/{}", args.id)).await,
        NetworkCommand::Create(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "description", args.description)?;
            set(&mut body, "network_type", args.network_type)?;
            set(&mut body, "host_id", args.host_id)?;
            set(&mut body, "cidr", args.cidr)?;
            set(&mut body, "vlan_id", args.vlan_id)?;
            set(&mut body, "dhcp_enabled", args.dhcp_enabled)?;
            set(&mut body, "uplink_interface", args.uplink_interface)?;
            set(&mut body, "gateway_host_id", args.gateway_host_id)?;
            client.post("/v1/networks", &body).await
        }
        NetworkCommand::Update(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "name", args.name)?;
            set(&mut body, "description", args.description)?;
            set(&mut body, "cidr", args.cidr)?;
            set(&mut body, "gateway", args.gateway)?;
            client
                .patch(&format!("/v1/networks/{}", args.id), &body)
                .await
        }
        NetworkCommand::Delete(args) => {
            confirm(args.yes, "Delete network?")?;
            client.delete(&format!("/v1/networks/{}", args.id)).await
        }
        NetworkCommand::Suggest(args) => {
            client
                .get(&format!("/v1/networks/suggest?host_id={}", args.host_id))
                .await
        }
        NetworkCommand::Interfaces(args) => {
            client
                .get(&format!("/v1/networks/interfaces?host_id={}", args.host_id))
                .await
        }
        NetworkCommand::Retry(args) => {
            client
                .post(&format!("/v1/networks/{}/retry", args.id), &json!({}))
                .await
        }
        NetworkCommand::Vms(args) => client.get(&format!("/v1/networks/{}/vms", args.id)).await,
    }
}

async fn run_storage_backend(client: &Client, cmd: StorageBackendCommand) -> Result<Value> {
    match cmd {
        StorageBackendCommand::List => client.get("/v1/storage_backends").await,
        StorageBackendCommand::Get(args) => {
            client
                .get(&format!("/v1/storage_backends/{}", args.id))
                .await
        }
    }
}

async fn run_user(client: &Client, cmd: UserCommand) -> Result<Value> {
    match cmd {
        UserCommand::List => client.get("/v1/users").await,
        UserCommand::Get(args) => client.get(&format!("/v1/users/{}", args.id)).await,
        UserCommand::Create(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "username", args.username)?;
            set(&mut body, "password", args.password)?;
            set(&mut body, "role", args.role)?;
            client.post("/v1/users", &body).await
        }
        UserCommand::Update(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "username", args.username)?;
            set(&mut body, "password", args.password)?;
            set(&mut body, "role", args.role)?;
            client.patch(&format!("/v1/users/{}", args.id), &body).await
        }
        UserCommand::Delete(args) => {
            confirm(args.yes, "Delete user?")?;
            client.delete(&format!("/v1/users/{}", args.id)).await
        }
    }
}

async fn run_license(client: &Client, cmd: LicenseCommand) -> Result<Value> {
    match cmd {
        LicenseCommand::Status => client.get("/v1/licensing/license/status").await,
        LicenseCommand::Activate(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "license_key", args.license_key)?;
            client.post("/v1/licensing/license/activate", &body).await
        }
        LicenseCommand::EulaStatus => client.get("/v1/licensing/eula/status").await,
        LicenseCommand::AcceptEula(args) => {
            let mut body = body_from_file(args.file)?;
            set(&mut body, "version", args.version)?;
            set(&mut body, "language", Some(args.language))?;
            client.post("/v1/licensing/eula/accept", &body).await
        }
    }
}

fn body_from_file(file: Option<PathBuf>) -> Result<Value> {
    match file {
        Some(path) => read_body_file(&path),
        None => Ok(Value::Object(Map::new())),
    }
}

fn required_body(file: Option<PathBuf>) -> Result<Value> {
    let Some(path) = file else {
        bail!("this command requires --file");
    };
    read_body_file(&path)
}

fn set<T: serde::Serialize>(body: &mut Value, key: &str, value: Option<T>) -> Result<()> {
    if let Some(value) = value {
        set_value(body, key, serde_json::to_value(value)?)
    } else {
        Ok(())
    }
}

fn set_value(body: &mut Value, key: &str, value: Value) -> Result<()> {
    let object = body
        .as_object_mut()
        .context("request body must be a JSON/YAML object")?;
    object.insert(key.to_string(), value);
    Ok(())
}

fn read_optional_text(path: Option<PathBuf>) -> Result<Option<String>> {
    path.map(|path| {
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))
    })
    .transpose()
}

fn query_path(path: &str, params: &[(&str, Option<String>)]) -> String {
    let pairs: Vec<String> = params
        .iter()
        .filter_map(|(key, value)| {
            value
                .as_ref()
                .map(|value| format!("{key}={}", urlencoding::encode(value)))
        })
        .collect();
    if pairs.is_empty() {
        path.to_string()
    } else {
        format!("{path}?{}", pairs.join("&"))
    }
}

fn confirm(yes: bool, prompt: &str) -> Result<()> {
    if yes {
        return Ok(());
    }

    print!("{prompt} Type 'yes' to continue: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    if input.trim() == "yes" {
        Ok(())
    } else {
        bail!("aborted")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn command_tree_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_representative_commands() {
        Cli::try_parse_from(["nqvm", "vm", "list"]).unwrap();
        Cli::try_parse_from(["nqvm", "vm", "get", "00000000-0000-0000-0000-000000000001"]).unwrap();
        Cli::try_parse_from([
            "nqvm",
            "vm",
            "shell",
            "00000000-0000-0000-0000-000000000001",
        ])
        .unwrap();
        Cli::try_parse_from([
            "nqvm",
            "container",
            "deploy",
            "--name",
            "web",
            "--image",
            "nginx",
        ])
        .unwrap();
        Cli::try_parse_from([
            "nqvm",
            "function",
            "invoke",
            "00000000-0000-0000-0000-000000000001",
            "--event",
            "{\"ok\":true}",
        ])
        .unwrap();
    }

    #[test]
    fn flag_values_override_file_body() {
        let mut body = json!({"name": "from-file", "vcpu": 1});
        set(&mut body, "name", Some("from-flag".to_string())).unwrap();
        assert_eq!(body["name"], "from-flag");
        assert_eq!(body["vcpu"], 1);
    }

    #[test]
    fn builds_query_path() {
        assert_eq!(
            query_path("/v1/images", &[("kind", Some("root fs".into()))]),
            "/v1/images?kind=root%20fs"
        );
        assert_eq!(query_path("/v1/images", &[("kind", None)]), "/v1/images");
    }
}
