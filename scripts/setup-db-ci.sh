#!/usr/bin/env bash
# Database setup script for CI environments
# Handles existing databases gracefully

set -euo pipefail

DB_NAME="${DB_NAME:-nexus}"
DB_USER="${DB_USER:-nexus}"
DB_PASSWORD="${DB_PASSWORD:-nexus}"

echo "Setting up PostgreSQL database for CI..."

# Start PostgreSQL service
sudo systemctl start postgresql || {
    echo "Warning: Failed to start PostgreSQL (may already be running)"
}

# Wait for PostgreSQL to be ready
for i in {1..30}; do
    if sudo -u postgres psql -c "SELECT 1;" >/dev/null 2>&1; then
        break
    fi
    echo "Waiting for PostgreSQL to be ready... ($i/30)"
    sleep 1
done

# Check if database already exists
if sudo -u postgres psql -lqt | cut -d \| -f 1 | grep -qw "$DB_NAME"; then
    echo "Database '$DB_NAME' already exists, skipping creation"
else
    echo "Creating database '$DB_NAME'..."
    sudo -u postgres psql -c "CREATE DATABASE $DB_NAME;" || {
        echo "Warning: Failed to create database (may already exist)"
    }
fi

# Check if user already exists
if sudo -u postgres psql -tAc "SELECT 1 FROM pg_roles WHERE rolname='$DB_USER'" | grep -q 1; then
    echo "User '$DB_USER' already exists, updating password..."
    sudo -u postgres psql -c "ALTER USER $DB_USER WITH ENCRYPTED PASSWORD '$DB_PASSWORD';" || {
        echo "Warning: Failed to update user password"
    }
else
    echo "Creating user '$DB_USER'..."
    sudo -u postgres psql -c "CREATE USER $DB_USER WITH ENCRYPTED PASSWORD '$DB_PASSWORD';" || {
        echo "Warning: Failed to create user (may already exist)"
    }
fi

# Grant privileges (idempotent - safe to run multiple times)
echo "Granting privileges..."
sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE $DB_NAME TO $DB_USER;" || true
sudo -u postgres psql -d "$DB_NAME" -c "GRANT ALL ON SCHEMA public TO $DB_USER;" || true
sudo -u postgres psql -d "$DB_NAME" -c "ALTER DATABASE $DB_NAME OWNER TO $DB_USER;" || true

echo "Database setup complete!"

