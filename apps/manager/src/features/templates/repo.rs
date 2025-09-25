use nexus_types::{CreateTemplateReq, Template, TemplateSpec};
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
