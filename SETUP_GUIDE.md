# Setup Guide - Frontend + Backend Integration

This guide explains how to run the updated frontend with the new Rust backend.

## Prerequisites

1. **Rust Backend Dependencies**:
   - Rust toolchain
   - PostgreSQL database
   - Environment variables configured

2. **Frontend Dependencies**:
   - Node.js (v18+)
   - pnpm package manager

## Quick Start

### 1. Start the Rust Backend

```bash
cd apps/manager

# Set required environment variables
export DATABASE_URL="postgresql://username:password@localhost/nexus"
export MANAGER_BIND="127.0.0.1:8080"  # Optional, defaults to this
export MANAGER_IMAGE_ROOT="/srv/images"  # Optional, defaults to /srv/images

# Run the backend
cargo run
```

The backend will start on `http://localhost:8080` with:
- API endpoints at `/v1/*`
- Swagger UI at `/docs`
- CORS enabled for frontend development

### 2. Start the Frontend

```bash
cd apps/frontend

# Install dependencies (if not already done)
pnpm install

# Start development server
pnpm dev
```

The frontend will start on `http://localhost:3000` and automatically connect to the backend.

## What's Fixed

### Backend Changes
- ✅ Added CORS support with `tower-http`
- ✅ Configured to allow all origins/methods/headers for development
- ✅ Backend serves API at `/v1/*` endpoints

### Frontend Changes
- ✅ Updated API base URL to point to `http://localhost:8080/v1`
- ✅ Fixed Next.js `themeColor` warning by moving to viewport export
- ✅ Updated all API calls to match new backend structure
- ✅ Added comprehensive TypeScript types for new backend
- ✅ Maintained backward compatibility during transition

## API Endpoints

The new backend provides these endpoints:

### VMs
- `GET /v1/vms` - List all VMs
- `POST /v1/vms` - Create new VM
- `GET /v1/vms/{id}` - Get specific VM
- `POST /v1/vms/{id}/stop` - Stop VM
- `DELETE /v1/vms/{id}` - Delete VM

### Templates (New Feature!)
- `GET /v1/templates` - List all templates
- `POST /v1/templates` - Create template
- `GET /v1/templates/{id}` - Get specific template
- `POST /v1/templates/{id}/instantiate` - Create VM from template

### Images
- `GET /v1/images` - List all images
- `GET /v1/images?kind=kernel` - List kernel images
- `GET /v1/images?kind=rootfs` - List rootfs images
- `POST /v1/images` - Register new image
- `DELETE /v1/images/{id}` - Delete image

### Snapshots
- `GET /v1/vms/{id}/snapshots` - List VM snapshots
- `POST /v1/vms/{id}/snapshots` - Create snapshot
- `POST /v1/snapshots/{id}/instantiate` - Create VM from snapshot
- `DELETE /v1/snapshots/{id}` - Delete snapshot

## Testing the Integration

1. **Start both services** as described above
2. **Open the frontend** at `http://localhost:3000`
3. **Check the dashboard** - it should load without CORS errors
4. **Try creating a VM** using the creation wizard
5. **Explore templates** (new feature) if you have any configured

## Troubleshooting

### CORS Errors
- Ensure the backend is running on port 8080
- Check that `tower-http` dependency was added to Cargo.toml
- Verify CORS layer is configured in main.rs

### Connection Refused
- Make sure the backend is running: `cargo run` in `apps/manager/`
- Check the backend is listening on the correct port (8080)
- Verify DATABASE_URL is set and database is accessible

### Frontend Build Errors
- Run `pnpm install` to ensure all dependencies are installed
- Check for TypeScript errors: `pnpm type-check`
- Clear Next.js cache: `rm -rf .next` and restart

### API Errors
- Check backend logs for detailed error messages
- Use the Swagger UI at `http://localhost:8080/docs` to test API endpoints directly
- Verify your database has the required tables (run migrations)

## Environment Variables

### Backend (.env or shell)
```bash
DATABASE_URL=postgresql://username:password@localhost/nexus
MANAGER_BIND=127.0.0.1:8080
MANAGER_IMAGE_ROOT=/srv/images
MANAGER_ALLOW_IMAGE_PATHS=true  # Optional: allow direct file paths
```

### Frontend
The frontend automatically connects to `http://localhost:8080/v1` in development.
To override, set: `NEXT_PUBLIC_API_BASE_URL=http://your-backend-url/v1`

## Next Steps

1. **Create some test data**:
   - Register kernel and rootfs images
   - Create VM templates
   - Test VM creation and management

2. **Explore new features**:
   - Template system for easy VM deployment
   - Unified image management
   - Improved snapshot workflow

3. **Production deployment**:
   - Configure proper CORS origins (not `Any`)
   - Set up reverse proxy if needed
   - Configure environment variables for production
