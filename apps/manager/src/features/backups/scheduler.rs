//! Per-volume cron scheduler. Wakes once a minute, checks every volume
//! that has backup_cron + backup_target_id set, dispatches a backup if due.

use crate::AppState;
use chrono::Utc;
use cron::Schedule;
use std::str::FromStr;
use std::time::Duration;
use uuid::Uuid;

pub async fn schedule_loop(state: AppState) {
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
        if let Err(e) = tick(&state).await {
            tracing::error!("scheduler tick: {e:#}");
        }
    }
}

type VolumeScheduleRow = (
    Uuid,
    Option<String>,
    Option<Uuid>,
    Option<chrono::DateTime<Utc>>,
);

async fn tick(st: &AppState) -> anyhow::Result<()> {
    let rows: Vec<VolumeScheduleRow> = sqlx::query_as(
        r#"SELECT v.id, v.backup_cron, v.backup_target_id,
                  (SELECT MAX(created_at) FROM backup b WHERE b.source_volume_id = v.id) AS last_backup
           FROM volume v
           WHERE v.backup_cron IS NOT NULL AND v.backup_target_id IS NOT NULL"#,
    )
    .fetch_all(&st.db)
    .await?;

    let now = Utc::now();
    for (volume_id, cron_str, target_id, last) in rows {
        let (Some(cron_str), Some(target_id)) = (cron_str, target_id) else {
            continue;
        };
        let Ok(schedule) = Schedule::from_str(&cron_str) else {
            tracing::warn!(volume_id=%volume_id, "invalid cron: {cron_str}");
            continue;
        };
        let after = last.unwrap_or(now - chrono::Duration::days(365));
        if let Some(next_fire) = schedule.after(&after).next() {
            if next_fire <= now {
                tracing::info!(volume_id=%volume_id, "scheduler firing backup");
                let st_cl = st.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::features::backups::service::create_backup(
                        &st_cl, volume_id, target_id,
                    )
                    .await
                    {
                        tracing::error!(volume_id=%volume_id, "scheduled backup failed: {e:#}");
                    }
                });
            }
        }
    }
    Ok(())
}
