#!/usr/bin/env bash
# Database setup for NQRust-MicroVM installer
# Configures PostgreSQL database

# Setup PostgreSQL service
setup_postgresql_service() {
    log_info "Setting up PostgreSQL service..."

    # Check if PostgreSQL is installed
    if ! command_exists psql; then
        log_error "PostgreSQL not installed"
        exit 1
    fi

    # Initialize database if needed (RHEL/Rocky)
    if [[ -f /usr/bin/postgresql-setup ]]; then
        if [[ ! -d /var/lib/pgsql/data/base ]]; then
            log_info "Initializing PostgreSQL database..."
            sudo postgresql-setup --initdb
        fi
    fi

    # Start PostgreSQL
    if ! systemctl is-active --quiet postgresql; then
        log_info "Starting PostgreSQL service..."
        sudo systemctl start postgresql
    fi

    # Enable on boot
    sudo systemctl enable postgresql >/dev/null 2>&1

    if systemctl is-active --quiet postgresql; then
        log_success "PostgreSQL service running"
    else
        log_error "Failed to start PostgreSQL"
        exit 1
    fi
}

# Create database and user
create_database() {
    local db_name="${DB_NAME:-nexus}"
    local db_user="${DB_USER:-nexus}"
    local db_password="${DB_PASSWORD:-}"

    log_info "Creating database and user..."

    # Generate password if not provided
    if [[ -z "$db_password" ]]; then
        db_password=$(generate_password 32)
        log_info "Generated database password"
    fi

    # Store password for later use
    export DB_PASSWORD="$db_password"

    # Check if database already exists
    if sudo -u postgres psql -lqt | cut -d \| -f 1 | grep -qw "$db_name"; then
        log_info "Database '$db_name' already exists"
    else
        log_info "Creating database '$db_name'..."
        sudo -u postgres psql -c "CREATE DATABASE $db_name;" || {
            log_error "Failed to create database"
            exit 1
        }
        log_success "Database created"
    fi

    # Check if user already exists
    if sudo -u postgres psql -tAc "SELECT 1 FROM pg_roles WHERE rolname='$db_user'" | grep -q 1; then
        log_info "User '$db_user' already exists, updating password..."
        sudo -u postgres psql -c "ALTER USER $db_user WITH ENCRYPTED PASSWORD '$db_password';"
    else
        log_info "Creating user '$db_user'..."
        sudo -u postgres psql -c "CREATE USER $db_user WITH ENCRYPTED PASSWORD '$db_password';"
        log_success "User created"
    fi

    # Grant privileges
    log_info "Granting privileges..."
    sudo -u postgres psql -c "GRANT ALL PRIVILEGES ON DATABASE $db_name TO $db_user;"

    # Grant schema privileges (required for SQLx migrations)
    sudo -u postgres psql -d "$db_name" -c "GRANT ALL ON SCHEMA public TO $db_user;"
    sudo -u postgres psql -d "$db_name" -c "ALTER DATABASE $db_name OWNER TO $db_user;"

    log_success "Database configured"
}

# Test database connection
test_database_connection() {
    local db_name="${DB_NAME:-nexus}"
    local db_user="${DB_USER:-nexus}"
    local db_host="${DB_HOST:-localhost}"
    local db_port="${DB_PORT:-5432}"
    local db_password="${DB_PASSWORD}"

    log_info "Testing database connection..."

    if PGPASSWORD="$db_password" psql -h "$db_host" -p "$db_port" -U "$db_user" -d "$db_name" -c "SELECT 1;" >/dev/null 2>&1; then
        log_success "Database connection successful"
        return 0
    else
        log_error "Database connection failed"
        log_error "Connection string: postgresql://$db_user@$db_host:$db_port/$db_name"
        return 1
    fi
}

# Configure PostgreSQL to accept local connections
configure_pg_hba() {
    log_info "Configuring PostgreSQL authentication..."

    local pg_version=$(psql --version | grep -oP '\d+' | head -1)
    local pg_hba_file=""

    # Find pg_hba.conf
    if [[ -f "/etc/postgresql/$pg_version/main/pg_hba.conf" ]]; then
        pg_hba_file="/etc/postgresql/$pg_version/main/pg_hba.conf"
    elif [[ -f "/var/lib/pgsql/data/pg_hba.conf" ]]; then
        pg_hba_file="/var/lib/pgsql/data/pg_hba.conf"
    else
        log_warn "Could not find pg_hba.conf, skipping configuration"
        return 0
    fi

    log_debug "pg_hba.conf: $pg_hba_file"

    # Backup original
    backup_file "$pg_hba_file"

    # Add local connection rules if not present
    if ! sudo grep -q "host.*nexus.*nexus.*127.0.0.1/32.*md5" "$pg_hba_file"; then
        log_info "Adding connection rules..."
        echo "# NQRust-MicroVM connections" | sudo tee -a "$pg_hba_file" >/dev/null
        echo "host    nexus           nexus           127.0.0.1/32            md5" | sudo tee -a "$pg_hba_file" >/dev/null
        echo "host    nexus           nexus           ::1/128                 md5" | sudo tee -a "$pg_hba_file" >/dev/null

        # Reload PostgreSQL
        sudo systemctl reload postgresql

        log_success "PostgreSQL authentication configured"
    else
        log_debug "Connection rules already present"
    fi
}

# Run database migrations
run_migrations() {
    log_info "Running database migrations..."

    local manager_dir="${PROJECT_ROOT:-/opt/nqrust-microvm}/apps/manager"

    if [[ ! -d "$manager_dir/migrations" ]]; then
        log_warn "Migrations directory not found, skipping"
        return 0
    fi

    # Set DATABASE_URL for migrations
    export DATABASE_URL="postgresql://${DB_USER:-nexus}:${DB_PASSWORD}@${DB_HOST:-localhost}:${DB_PORT:-5432}/${DB_NAME:-nexus}"

    # Run migrations using sqlx
    cd "$manager_dir"

    if sqlx migrate run; then
        log_success "Database migrations completed"
    else
        log_error "Database migrations failed"
        exit 1
    fi
}

# Setup PostgreSQL (main function)
setup_postgresql() {
    log_info "Setting up PostgreSQL..."

    # Check if using external database
    if [[ "${DB_TYPE:-local}" == "remote" ]]; then
        log_info "Using remote PostgreSQL database"
        log_info "Host: ${DB_HOST:-localhost}"

        if [[ -z "${DB_PASSWORD}" ]]; then
            if [[ "$NON_INTERACTIVE" != "true" ]]; then
                read -sp "Enter database password: " DB_PASSWORD
                echo
                export DB_PASSWORD
            else
                log_error "DB_PASSWORD required for remote database"
                exit 1
            fi
        fi

        # Test connection only
        if ! test_database_connection; then
            log_error "Cannot connect to remote database"
            exit 1
        fi
    else
        # Local PostgreSQL setup
        setup_postgresql_service
        configure_pg_hba
        create_database
        test_database_connection || exit 1
    fi

    # Save connection info for later
    DB_CONNECTION_STRING="postgresql://${DB_USER:-nexus}:${DB_PASSWORD}@${DB_HOST:-localhost}:${DB_PORT:-5432}/${DB_NAME:-nexus}"
    export DB_CONNECTION_STRING

    log_success "PostgreSQL setup complete"
}

# Show database information
show_database_info() {
    echo ""
    log_info "Database Information:"
    echo "  Type:     ${DB_TYPE:-local}"
    echo "  Host:     ${DB_HOST:-localhost}"
    echo "  Port:     ${DB_PORT:-5432}"
    echo "  Database: ${DB_NAME:-nexus}"
    echo "  User:     ${DB_USER:-nexus}"
    if [[ "${DB_TYPE:-local}" == "local" ]]; then
        echo "  Password: ${DB_PASSWORD:0:8}... (saved in config)"
    fi
    echo ""
}

# Export functions
export -f setup_postgresql_service create_database test_database_connection
export -f configure_pg_hba run_migrations setup_postgresql
export -f show_database_info
