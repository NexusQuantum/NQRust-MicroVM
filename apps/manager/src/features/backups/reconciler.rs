//! Periodically marks `running` backups older than 24h as `failed`.

use crate::features::backups::repo::BackupRepository;
use sqlx::PgPool;

const STALE_MINUTES: i64 = 24 * 60;

pub async fn reconcile_loop(pool: PgPool) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5 * 60)).await;
        let repo = BackupRepository::new(pool.clone());
        match repo.list_stale_running(STALE_MINUTES).await {
            Ok(rows) => {
                for r in rows {
                    let _ = repo
                        .mark_failed(
                            r.id,
                            &format!(
                                "marked failed by reconciler: status was 'running' for >{STALE_MINUTES} minutes"
                            ),
                        )
                        .await;
                    tracing::warn!(backup_id=%r.id, "reconciler aged stuck 'running' to 'failed'");
                }
            }
            Err(e) => tracing::error!("reconciler: {e}"),
        }
    }
}
