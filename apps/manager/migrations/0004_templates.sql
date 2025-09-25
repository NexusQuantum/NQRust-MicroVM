CREATE TABLE IF NOT EXISTS template (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    spec_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE vm ADD COLUMN IF NOT EXISTS template_id UUID;
ALTER TABLE vm
    ADD CONSTRAINT fk_vm_template
    FOREIGN KEY (template_id) REFERENCES template(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_vm_template ON vm(template_id);
