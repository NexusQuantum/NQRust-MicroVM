-- License table to cache verification results for grace period fallback
CREATE TABLE license (
    id               UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    license_key      VARCHAR(255) NOT NULL,
    status           VARCHAR(50) NOT NULL DEFAULT 'unknown',
    customer_name    VARCHAR(255),
    product          VARCHAR(255),
    product_id       VARCHAR(255),
    customer_id      VARCHAR(255),
    features         JSONB DEFAULT '[]'::jsonb,
    expires_at       DATE,
    verified_at      TIMESTAMPTZ,
    activations      INTEGER,
    max_activations  INTEGER,
    cached_response  JSONB,
    device_id        VARCHAR(64),
    is_offline       BOOLEAN NOT NULL DEFAULT false,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);
