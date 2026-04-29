use nexus_types::{CreateTemplateReq, Template, TemplateSpec, UpdateTemplateReq};
use sqlx::{error::BoxDynError, PgPool};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct TemplateRow {
    id: Uuid,
    name: String,
    spec_json: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<TemplateRow> for Template {
    type Error = sqlx::Error;

    fn try_from(row: TemplateRow) -> Result<Self, Self::Error> {
        let spec: TemplateSpec = serde_json::from_value(row.spec_json).map_err(|err| {
            let boxed: BoxDynError = Box::new(err);
            sqlx::Error::Decode(boxed)
        })?;
        Ok(Template {
            id: row.id,
            name: row.name,
            spec,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

pub async fn insert(db: &PgPool, req: &CreateTemplateReq) -> sqlx::Result<Template> {
    let spec_json = serde_json::to_value(&req.spec).map_err(|err| {
        let boxed: BoxDynError = Box::new(err);
        sqlx::Error::Decode(boxed)
    })?;

    let row = sqlx::query_as::<_, TemplateRow>(
        r#"
        INSERT INTO template (id, name, spec_json)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(&req.name)
    .bind(spec_json)
    .fetch_one(db)
    .await?;

    row.try_into()
}

pub async fn list(db: &PgPool) -> sqlx::Result<Vec<Template>> {
    let rows = sqlx::query_as::<_, TemplateRow>(
        r#"
        SELECT id, name, spec_json, created_at, updated_at
        FROM template
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(db)
    .await?;

    rows.into_iter().map(Template::try_from).collect()
}

pub async fn get(db: &PgPool, id: Uuid) -> sqlx::Result<Template> {
    let row = sqlx::query_as::<_, TemplateRow>(
        r#"
        SELECT id, name, spec_json, created_at, updated_at
        FROM template
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_one(db)
    .await?;

    row.try_into()
}

pub async fn update(db: &PgPool, id: Uuid, req: &UpdateTemplateReq) -> sqlx::Result<Template> {
    let spec_json = serde_json::to_value(&req.spec).map_err(|err| {
        let boxed: BoxDynError = Box::new(err);
        sqlx::Error::Decode(boxed)
    })?;

    let row = sqlx::query_as::<_, TemplateRow>(
        r#"
        UPDATE template
        SET name = $2, spec_json = $3, updated_at = now()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&req.name)
    .bind(spec_json)
    .fetch_one(db)
    .await?;

    row.try_into()
}

pub async fn delete(db: &PgPool, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM template WHERE id = $1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_row(id: Uuid, spec_json: serde_json::Value) -> TemplateRow {
        let now = chrono::Utc::now();
        TemplateRow {
            id,
            name: "ubuntu".into(),
            spec_json,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn template_row_try_from_decodes_full_spec() {
        let id = Uuid::new_v4();
        let kernel_image_id = Uuid::new_v4();
        let rootfs_image_id = Uuid::new_v4();
        let row = sample_row(
            id,
            json!({
                "vcpu": 4,
                "mem_mib": 4096,
                "kernel_image_id": kernel_image_id,
                "rootfs_image_id": rootfs_image_id,
                "kernel_path": "/srv/k",
                "rootfs_path": "/srv/r",
                "rootfs_size_mb": 8192,
            }),
        );

        let template: Template = row.try_into().expect("decode should succeed");
        assert_eq!(template.id, id);
        assert_eq!(template.name, "ubuntu");
        assert_eq!(template.spec.vcpu, 4);
        assert_eq!(template.spec.mem_mib, 4096);
        assert_eq!(template.spec.kernel_image_id, Some(kernel_image_id));
        assert_eq!(template.spec.rootfs_image_id, Some(rootfs_image_id));
        assert_eq!(template.spec.kernel_path.as_deref(), Some("/srv/k"));
        assert_eq!(template.spec.rootfs_path.as_deref(), Some("/srv/r"));
        assert_eq!(template.spec.rootfs_size_mb, Some(8192));
    }

    #[test]
    fn template_row_try_from_decodes_minimal_spec_with_defaults() {
        let row = sample_row(
            Uuid::new_v4(),
            json!({
                "vcpu": 1,
                "mem_mib": 256,
            }),
        );

        let template: Template = row.try_into().expect("minimal spec should decode");
        assert_eq!(template.spec.vcpu, 1);
        assert_eq!(template.spec.mem_mib, 256);
        assert!(template.spec.kernel_image_id.is_none());
        assert!(template.spec.rootfs_image_id.is_none());
        assert!(template.spec.kernel_path.is_none());
        assert!(template.spec.rootfs_path.is_none());
        assert!(template.spec.rootfs_size_mb.is_none());
    }

    #[test]
    fn template_row_try_from_rejects_malformed_spec_json() {
        let row = sample_row(
            Uuid::new_v4(),
            json!({
                "mem_mib": 512,
            }),
        );

        let result: Result<Template, _> = row.try_into();
        assert!(result.is_err(), "missing required vcpu must fail decode");
        match result {
            Err(sqlx::Error::Decode(_)) => {}
            Err(other) => panic!("expected Decode error, got {other:?}"),
            Ok(_) => panic!("expected error"),
        }
    }
}
