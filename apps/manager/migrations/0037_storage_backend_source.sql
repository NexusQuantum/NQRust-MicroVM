-- Track whether a storage_backend row was seeded from storage.toml or
-- created via the admin UI. The startup TOML reconciler only soft-deletes
-- rows with source='toml' that are no longer in the file; UI-created rows
-- (source='ui') survive restarts.
ALTER TABLE storage_backend
    ADD COLUMN source TEXT NOT NULL DEFAULT 'toml'
    CHECK (source IN ('toml', 'ui'));
