use std::path::{Component, Path, PathBuf};

use nexus_types::{CreateImageReq, Image, ImageFilter};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone)]
pub struct ImageRepository {
    pool: PgPool,
    root: PathBuf,
}

impl ImageRepository {
    pub fn new(pool: PgPool, root: impl Into<PathBuf>) -> Self {
        let root_path: PathBuf = root.into();
        let root_path = if root_path.is_absolute() {
            root_path
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(root_path)
        };
        Self {
            pool,
            root: root_path,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn is_path_allowed(&self, path: &Path) -> bool {
        path_within_root(&self.root, path)
    }

    pub async fn insert(&self, req: &CreateImageReq) -> Result<Image, ImageRepoError> {
        if !self.is_path_allowed(Path::new(&req.host_path)) {
            return Err(ImageRepoError::InvalidPath(req.host_path.clone()));
        }

        let row = sqlx::query_as::<_, ImageRow>(
            r#"
            INSERT INTO image (id, kind, name, host_path, sha256, size, project)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(&req.kind)
        .bind(&req.name)
        .bind(&req.host_path)
        .bind(&req.sha256)
        .bind(req.size)
        .bind(&req.project)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn list(&self, filter: &ImageFilter) -> Result<Vec<Image>, ImageRepoError> {
        let rows = sqlx::query_as::<_, ImageRow>(
            r#"
            SELECT id, kind, name, host_path, sha256, size, project, created_at, updated_at
            FROM image
            WHERE ($1::text IS NULL OR kind = $1)
              AND ($2::text IS NULL OR project = $2)
              AND ($3::text IS NULL OR name ILIKE $3)
            ORDER BY created_at DESC
            "#,
        )
        .bind(filter.kind.as_ref())
        .bind(filter.project.as_ref())
        .bind(filter.name.as_ref().map(|name| format!("%{}%", name)))
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(&self, id: Uuid) -> Result<Image, ImageRepoError> {
        let row = sqlx::query_as::<_, ImageRow>(
            r#"
            SELECT id, kind, name, host_path, sha256, size, project, created_at, updated_at
            FROM image
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn delete(&self, id: Uuid) -> Result<(), ImageRepoError> {
        sqlx::query("DELETE FROM image WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ImageRepoError {
    #[error("image path '{0}' is not within the configured root")]
    InvalidPath(String),
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
}

#[derive(sqlx::FromRow)]
struct ImageRow {
    id: Uuid,
    kind: String,
    name: String,
    host_path: String,
    sha256: String,
    size: i64,
    project: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<ImageRow> for Image {
    fn from(row: ImageRow) -> Self {
        Image {
            id: row.id,
            kind: row.kind,
            name: row.name,
            host_path: row.host_path,
            sha256: row.sha256,
            size: row.size,
            project: row.project,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

fn path_within_root(root: &Path, candidate: &Path) -> bool {
    if !candidate.is_absolute() {
        return false;
    }

    if candidate
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return false;
    }

    candidate.starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_paths() {
        assert!(!path_within_root(
            Path::new("/srv/images"),
            Path::new("relative/path")
        ));
    }

    #[test]
    fn rejects_parent_dirs() {
        assert!(!path_within_root(
            Path::new("/srv/images"),
            Path::new("/srv/images/../etc/passwd"),
        ));
    }

    #[test]
    fn accepts_paths_under_root() {
        assert!(path_within_root(
            Path::new("/srv/images"),
            Path::new("/srv/images/vmlinux"),
        ));
    }
}
