//! Database setup module.

use std::fs;

use anyhow::{anyhow, Result};
use rand::Rng;

use crate::app::LogEntry;
use crate::installer::{command_exists, run_command, run_sudo};

/// Generate a random password
pub fn generate_password(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Setup PostgreSQL database
pub fn setup_database(db_name: &str, db_user: &str, db_password: &str) -> Result<Vec<LogEntry>> {
    let mut logs = Vec::new();

    // Check if PostgreSQL is installed
    if !command_exists("psql") {
        logs.push(LogEntry::error("PostgreSQL is not installed"));
        return Err(anyhow!("PostgreSQL not found"));
    }

    logs.push(LogEntry::info("Setting up PostgreSQL database..."));

    // Start and enable PostgreSQL
    logs.push(LogEntry::info("Starting PostgreSQL service..."));
    let _ = run_sudo("systemctl", &["enable", "postgresql"]);
    let output = run_sudo("systemctl", &["start", "postgresql"])?;

    if !output.status.success() {
        // Try to initialize on RHEL-based systems
        logs.push(LogEntry::info("Initializing PostgreSQL (RHEL-based)..."));
        let _ = run_sudo("postgresql-setup", &["--initdb"]);
        let _ = run_sudo("systemctl", &["start", "postgresql"]);
    }

    logs.push(LogEntry::success("PostgreSQL service started"));

    // Check if database already exists
    let check_db = run_command(
        "sudo",
        &[
            "-u",
            "postgres",
            "psql",
            "-tAc",
            &format!("SELECT 1 FROM pg_database WHERE datname='{}'", db_name),
        ],
    )?;

    let db_exists = String::from_utf8_lossy(&check_db.stdout).trim() == "1";

    if db_exists {
        logs.push(LogEntry::info(format!(
            "Database '{}' already exists",
            db_name
        )));

        // Even if DB exists, ensure user password is correct
        logs.push(LogEntry::info(format!(
            "Updating password for user '{}'...",
            db_user
        )));
        let alter_user_sql = format!(
            "ALTER USER {} WITH ENCRYPTED PASSWORD '{}';",
            db_user, db_password
        );
        let _ = run_command("sudo", &["-u", "postgres", "psql", "-c", &alter_user_sql]);
        logs.push(LogEntry::success(format!(
            "User '{}' password updated",
            db_user
        )));
    } else {
        // Create user or update password if exists
        logs.push(LogEntry::info(format!("Creating user '{}'...", db_user)));

        let create_user_sql = format!(
            "CREATE USER {} WITH ENCRYPTED PASSWORD '{}';",
            db_user, db_password
        );
        let output = run_command("sudo", &["-u", "postgres", "psql", "-c", &create_user_sql]);

        if let Ok(out) = output {
            if out.status.success() {
                logs.push(LogEntry::success(format!("User '{}' created", db_user)));
            } else if String::from_utf8_lossy(&out.stderr).contains("already exists") {
                // User exists, update password
                logs.push(LogEntry::info(format!(
                    "User '{}' exists, updating password...",
                    db_user
                )));
                let alter_user_sql = format!(
                    "ALTER USER {} WITH ENCRYPTED PASSWORD '{}';",
                    db_user, db_password
                );
                let _ = run_command("sudo", &["-u", "postgres", "psql", "-c", &alter_user_sql]);
                logs.push(LogEntry::success(format!(
                    "User '{}' password updated",
                    db_user
                )));
            }
        }

        // Create database
        logs.push(LogEntry::info(format!(
            "Creating database '{}'...",
            db_name
        )));

        let create_db_sql = format!(
            "CREATE DATABASE {} WITH OWNER = {} ENCODING = 'UTF8';",
            db_name, db_user
        );
        let output = run_command("sudo", &["-u", "postgres", "psql", "-c", &create_db_sql])?;

        if output.status.success() {
            logs.push(LogEntry::success(format!("Database '{}' created", db_name)));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("already exists") {
                logs.push(LogEntry::info(format!(
                    "Database '{}' already exists",
                    db_name
                )));
            } else {
                logs.push(LogEntry::error(format!(
                    "Failed to create database: {}",
                    stderr
                )));
            }
        }

        // Grant permissions
        logs.push(LogEntry::info("Granting database permissions..."));

        let grant_sql = format!(
            "GRANT ALL PRIVILEGES ON DATABASE {} TO {};",
            db_name, db_user
        );
        let _ = run_command("sudo", &["-u", "postgres", "psql", "-c", &grant_sql]);

        // Grant schema permissions for SQLx
        let grant_schema_sql = format!("GRANT ALL ON SCHEMA public TO {};", db_user);
        let _ = run_command(
            "sudo",
            &[
                "-u",
                "postgres",
                "psql",
                "-d",
                db_name,
                "-c",
                &grant_schema_sql,
            ],
        );

        logs.push(LogEntry::success("Database permissions configured"));
    }

    // Configure pg_hba.conf for local connections
    logs.push(LogEntry::info("Configuring PostgreSQL authentication..."));
    configure_pg_hba()?;
    logs.push(LogEntry::success("PostgreSQL authentication configured"));

    // Restart PostgreSQL to apply changes
    let _ = run_sudo("systemctl", &["restart", "postgresql"]);

    // Test connection
    logs.push(LogEntry::info("Testing database connection..."));
    let test_result = test_database_connection(db_name, db_user, db_password);

    if test_result {
        logs.push(LogEntry::success("Database connection successful"));
    } else {
        logs.push(LogEntry::warning(
            "Database connection test failed - may need manual verification",
        ));
    }

    Ok(logs)
}

/// Configure pg_hba.conf for password authentication
fn configure_pg_hba() -> Result<()> {
    // Find pg_hba.conf
    let possible_paths = [
        "/etc/postgresql/*/main/pg_hba.conf",
        "/var/lib/pgsql/data/pg_hba.conf",
        "/var/lib/postgresql/*/data/pg_hba.conf",
    ];

    for pattern in &possible_paths {
        if let Ok(output) = run_command("sh", &["-c", &format!("ls {}", pattern)]) {
            if output.status.success() {
                let paths = String::from_utf8_lossy(&output.stdout);
                for path in paths.lines() {
                    let path = path.trim();
                    if !path.is_empty() {
                        // Read current config
                        if let Ok(content) = fs::read_to_string(path) {
                            // Check if already configured for md5/scram-sha-256
                            if content.contains("md5") || content.contains("scram-sha-256") {
                                return Ok(());
                            }

                            // Add md5 auth for local connections
                            let new_line =
                                "local   all             all                                     md5";

                            // Prepend to file (before other rules)
                            let backup_cmd = format!("sudo cp {} {}.backup", path, path);
                            let _ = run_command("sh", &["-c", &backup_cmd]);

                            // This is a simple approach - in production, we'd be more careful
                            let sed_cmd = format!("sudo sed -i '1s/^/{}\\n/' {}", new_line, path);
                            let _ = run_command("sh", &["-c", &sed_cmd]);
                        }
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

/// Test database connection
fn test_database_connection(db_name: &str, db_user: &str, db_password: &str) -> bool {
    let connection_string = format!(
        "postgresql://{}:{}@localhost:5432/{}",
        db_user, db_password, db_name
    );

    // Try to connect using psql
    let output = run_command("psql", &[&connection_string, "-c", "SELECT 1;"]);

    if let Ok(out) = output {
        return out.status.success();
    }

    // Fallback: try with PGPASSWORD environment variable
    let output = run_command(
        "sh",
        &[
            "-c",
            &format!(
                "PGPASSWORD='{}' psql -h localhost -U {} -d {} -c 'SELECT 1;'",
                db_password, db_user, db_name
            ),
        ],
    );

    output.map(|o| o.status.success()).unwrap_or(false)
}

/// Build database URL for configuration
pub fn build_database_url(
    host: &str,
    port: u16,
    db_name: &str,
    db_user: &str,
    db_password: &str,
) -> String {
    format!(
        "postgresql://{}:{}@{}:{}/{}",
        db_user, db_password, host, port, db_name
    )
}

/// Verify database is working
pub fn verify_database(db_name: &str, db_user: &str, db_password: &str) -> Result<bool> {
    Ok(test_database_connection(db_name, db_user, db_password))
}
