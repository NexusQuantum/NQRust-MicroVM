use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct LicensingRepository {
    pool: PgPool,
}

/// Row from the `license` cache table.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CachedLicense {
    pub license_key: String,
    pub status: String,
    pub customer_name: Option<String>,
    pub product: Option<String>,
    pub features: Option<serde_json::Value>,
    pub expires_at: Option<chrono::NaiveDate>,
    pub verified_at: Option<chrono::DateTime<chrono::Utc>>,
    pub activations: Option<i32>,
    pub max_activations: Option<i32>,
    pub device_id: Option<String>,
    pub is_offline: bool,
}

impl LicensingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ── EULA methods ──

    pub async fn get_latest_acceptance(
        &self,
        user_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        let row: Option<String> = sqlx::query_scalar(
            r#"
            SELECT eula_version
            FROM eula_acceptances
            WHERE user_id = $1
            ORDER BY accepted_at DESC
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    pub async fn has_accepted_version(
        &self,
        user_id: Uuid,
        version: &str,
    ) -> Result<bool, sqlx::Error> {
        let exists: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 
                FROM eula_acceptances 
                WHERE user_id = $1 AND eula_version = $2
            )
            "#,
        )
        .bind(user_id)
        .bind(version)
        .fetch_optional(&self.pool)
        .await?;

        Ok(exists.unwrap_or(false))
    }

    pub async fn record_acceptance(
        &self,
        user_id: Uuid,
        version: &str,
        language: &str,
        ip_address: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO eula_acceptances (user_id, eula_version, language, ip_address)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, eula_version) 
            DO UPDATE SET 
                language = EXCLUDED.language,
                ip_address = EXCLUDED.ip_address,
                accepted_at = now()
            "#,
        )
        .bind(user_id)
        .bind(version)
        .bind(language)
        .bind(ip_address)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ── App-level EULA methods ──

    /// Get the latest app-level EULA acceptance version.
    pub async fn get_app_acceptance(&self) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT eula_version FROM app_eula_acceptance ORDER BY accepted_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Record an app-level EULA acceptance.
    pub async fn record_app_acceptance(
        &self,
        version: &str,
        language: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO app_eula_acceptance (eula_version, language) VALUES ($1, $2)")
            .bind(version)
            .bind(language)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── License cache methods ──

    /// Upsert the cached license verification result.
    #[allow(clippy::too_many_arguments)]
    pub async fn upsert_license(
        &self,
        license_key: &str,
        status: &str,
        customer_name: Option<&str>,
        product: Option<&str>,
        features: serde_json::Value,
        expires_at: Option<chrono::NaiveDate>,
        activations: Option<i32>,
        max_activations: Option<i32>,
        device_id: &str,
        is_offline: bool,
    ) -> Result<(), sqlx::Error> {
        // Delete any existing row and insert fresh
        sqlx::query("DELETE FROM license")
            .execute(&self.pool)
            .await?;

        sqlx::query(
            r#"
            INSERT INTO license (
                license_key, status, customer_name, product,
                features, expires_at, verified_at,
                activations, max_activations, device_id, is_offline
            ) VALUES ($1, $2, $3, $4, $5, $6, now(), $7, $8, $9, $10)
            "#,
        )
        .bind(license_key)
        .bind(status)
        .bind(customer_name)
        .bind(product)
        .bind(features)
        .bind(expires_at)
        .bind(activations)
        .bind(max_activations)
        .bind(device_id)
        .bind(is_offline)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the latest cached license verification.
    pub async fn get_latest_license(&self) -> Result<Option<CachedLicense>, sqlx::Error> {
        let row: Option<CachedLicense> = sqlx::query_as(
            r#"
            SELECT
                license_key,
                status,
                customer_name,
                product,
                features,
                expires_at,
                verified_at,
                activations,
                max_activations,
                device_id,
                is_offline
            FROM license
            ORDER BY updated_at DESC
            LIMIT 1
            "#,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }
}
