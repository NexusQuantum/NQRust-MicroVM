# Settings Page Implementation Status

## ✅ COMPLETED - Backend Implementation

### 1. Database Schema
**File**: `apps/manager/migrations/0022_user_preferences.sql`
- Added `preferences` JSONB column for flexible user settings
- Added `timezone` VARCHAR(50) for user timezone preference
- Added `theme` VARCHAR(20) for dark/light mode
- Added `avatar_path` TEXT for avatar storage location
- Created GIN index on preferences for fast JSON queries

### 2. Shared Types
**File**: `crates/nexus-types/src/lib.rs`

Extended `User` struct:
```rust
pub struct User {
    pub avatar_path: Option<String>,
    pub timezone: Option<String>,
    pub theme: Option<String>,
    // ... existing fields
}
```

New preference types:
- `NotificationPreferences` - email, browser, desktop notifications
- `VmDefaults` - default VM creation settings (vcpu, mem_mib, disk_gb)
- `UserPreferences` - complete preferences structure
- `UpdatePreferencesRequest` - partial update support
- `UpdateProfileRequest` - username updates
- `ChangePasswordRequest` - password change with verification

### 3. Repository Methods
**File**: `apps/manager/src/features/users/repo.rs`

Implemented methods:
- `get_preferences(user_id)` - Fetch preferences from JSONB + columns
- `update_preferences(user_id, req)` - Merge and save preferences
- `update_profile(user_id, username)` - Update username
- `change_password(user_id, current, new)` - Verify old password, hash new
- `get_avatar_path(user_id)` - Get avatar file path
- `set_avatar_path(user_id, path)` - Update avatar path in DB
- `delete_avatar(user_id)` - Remove avatar reference

### 4. API Endpoints
**File**: `apps/manager/src/features/users/routes.rs` (655 lines)

#### Preferences:
- `GET /v1/auth/me/preferences` - Get current user preferences
- `PATCH /v1/auth/me/preferences` - Update preferences (partial updates supported)

#### Profile:
- `GET /v1/auth/me/profile` - Get current user profile
- `PATCH /v1/auth/me/profile` - Update username
- `POST /v1/auth/me/password` - Change password (requires current password)

#### Avatar:
- `POST /v1/auth/me/avatar` - Upload avatar (multipart/form-data)
  - Validates PNG format
  - Max 2MB file size
  - Automatic resize to 500x500px using `image` crate
  - Saves to `/srv/images/avatars/{user_id}.png`
- `GET /v1/auth/me/avatar` - Get own avatar image
- `GET /v1/users/{id}/avatar` - Get any user's avatar (admin only)
- `DELETE /v1/auth/me/avatar` - Delete avatar

### 5. Dependencies Added
**File**: `apps/manager/Cargo.toml`
- `image = "0.24"` - Image processing and resizing
- `tokio-util = { version = "0.7", features = ["io"] }` - Multipart handling

---

## ✅ COMPLETED - Frontend Types & API Client

### 1. Frontend Types
**File**: `apps/ui/lib/types/index.ts`

Extended `User` interface:
```typescript
export interface User {
    avatar_path?: string;
    timezone?: string;
    theme?: string;
    // ... existing fields
}
```

New types:
- `NotificationPreferences`
- `VmDefaults`
- `UserPreferences`
- `GetPreferencesResponse`
- `UpdatePreferencesRequest`
- `UpdateProfileRequest`
- `ChangePasswordRequest`

### 2. API Client Methods
**File**: `apps/ui/lib/api/facade.ts`

Implemented methods:
- `getPreferences()` - Fetch user preferences
- `updatePreferences(params)` - Update preferences
- `getProfile()` - Get user profile
- `updateProfile(params)` - Update username
- `changePassword(params)` - Change password
- `uploadAvatar(file)` - Upload avatar with FormData
- `getAvatarUrl(userId)` - Get avatar URL for any user
- `getMyAvatarUrl()` - Get current user's avatar URL
- `deleteAvatar()` - Delete avatar

---

## 🚧 REMAINING - Frontend Implementation

### 1. React Query Hooks
**File**: `apps/ui/lib/queries.ts` (TO DO)

Need to add:
```typescript
// Preferences
export const usePreferences = () => useQuery({
    queryKey: queryKeys.preferences(),
    queryFn: () => facadeApi.getPreferences(),
});

export const useUpdatePreferences = () => useMutation({
    mutationFn: (params: UpdatePreferencesRequest) => facadeApi.updatePreferences(params),
    onSuccess: () => queryClient.invalidateQueries(queryKeys.preferences()),
});

// Profile
export const useProfile = () => useQuery({
    queryKey: queryKeys.profile(),
    queryFn: () => facadeApi.getProfile(),
});

export const useUpdateProfile = () => useMutation({
    mutationFn: (params: UpdateProfileRequest) => facadeApi.updateProfile(params),
    onSuccess: () => {
        queryClient.invalidateQueries(queryKeys.profile());
        queryClient.invalidateQueries(queryKeys.currentUser());
    },
});

export const useChangePassword = () => useMutation({
    mutationFn: (params: ChangePasswordRequest) => facadeApi.changePassword(params),
});

// Avatar
export const useUploadAvatar = () => useMutation({
    mutationFn: (file: File) => facadeApi.uploadAvatar(file),
    onSuccess: () => queryClient.invalidateQueries(queryKeys.profile()),
});

export const useDeleteAvatar = () => useMutation({
    mutationFn: () => facadeApi.deleteAvatar(),
    onSuccess: () => queryClient.invalidateQueries(queryKeys.profile()),
});
```

### 2. Avatar Component
**File**: `apps/ui/components/user/avatar.tsx` (TO CREATE)

Reusable avatar component:
- Displays user avatar from `avatar_path`
- Falls back to initials if no avatar
- Supports different sizes (sm, md, lg, xl)
- Used in topbar, settings, user table

### 3. Settings Page Refactor
**File**: `apps/ui/app/(dashboard)/settings/page.tsx` (TO MODIFY)

Current issues:
- Uses localStorage only
- No backend persistence
- Changes lost on logout

Needs:
- Load preferences from `usePreferences()`
- Save to backend via `useUpdatePreferences()`
- Sync theme with `next-themes`
- Loading states while fetching/saving
- Success/error toasts

### 4. Add Account Tab
**File**: `apps/ui/app/(dashboard)/settings/page.tsx` (TO ADD)

New "Account" tab with:
- **Avatar Upload Section**:
  - Current avatar preview (500x500px circle)
  - Upload button (accepts PNG, max 2MB)
  - Client-side preview before upload
  - Remove avatar button
  - Upload progress indicator

- **Profile Section**:
  - Username change form
  - Display current username
  - Validation (non-empty, unique)

- **Password Section**:
  - Current password field
  - New password field
  - Confirm new password field
  - Password strength indicator
  - Validation (match confirmation)

- **Account Info** (read-only):
  - User role badge
  - Account creation date
  - Last login time

---

## Implementation Priority

### Phase 1: Core Functionality (Next Steps)
1. Add React Query hooks to `queries.ts`
2. Create Avatar component
3. Refactor settings page to use backend preferences

### Phase 2: Account Management
4. Add Account tab with avatar upload
5. Add profile management (username change)
6. Add password change functionality

### Phase 3: Polish
7. Add loading states and error handling
8. Add success/error toasts
9. Test all functionality end-to-end
10. Update topbar to show avatar

---

## Testing Checklist

### Backend
- [ ] Run migrations: `cd apps/manager && sqlx migrate run`
- [ ] Build backend: `cargo build -p manager`
- [ ] Test preferences endpoints with curl
- [ ] Test avatar upload with max file size
- [ ] Test avatar resize to 500x500px
- [ ] Test password change with wrong current password

### Frontend
- [ ] Preferences load from backend
- [ ] Preferences save to backend
- [ ] Theme syncs with next-themes
- [ ] Avatar upload works
- [ ] Avatar displays correctly
- [ ] Avatar URL generation works
- [ ] Username change updates everywhere
- [ ] Password change validates current password

---

## API Endpoints Summary

### Auth (Authenticated Users)
```
GET    /v1/auth/me/preferences      - Get preferences
PATCH  /v1/auth/me/preferences      - Update preferences
GET    /v1/auth/me/profile          - Get profile
PATCH  /v1/auth/me/profile          - Update profile (username)
POST   /v1/auth/me/password         - Change password
POST   /v1/auth/me/avatar           - Upload avatar
GET    /v1/auth/me/avatar           - Get own avatar
DELETE /v1/auth/me/avatar           - Delete avatar
```

### Users (Admin Only)
```
GET    /v1/users/{id}/avatar        - Get user avatar
```

---

## File Locations

### Backend
- Migration: `apps/manager/migrations/0022_user_preferences.sql`
- Types: `crates/nexus-types/src/lib.rs` (lines 1082-1209)
- Repo: `apps/manager/src/features/users/repo.rs` (lines 15-27, 310-467)
- Routes: `apps/manager/src/features/users/routes.rs` (lines 1-15, 287-655)
- Router: `apps/manager/src/features/users/mod.rs` (lines 9-18, 26)

### Frontend
- Types: `apps/ui/lib/types/index.ts` (lines 771-851)
- API: `apps/ui/lib/api/facade.ts` (lines 695-741)
- Queries: `apps/ui/lib/queries.ts` (TO ADD)
- Avatar: `apps/ui/components/user/avatar.tsx` (TO CREATE)
- Settings: `apps/ui/app/(dashboard)/settings/page.tsx` (TO MODIFY)

---

## Notes

### Avatar Storage
- Path: `/srv/images/avatars/`
- Format: `{user_id}.png`
- Size: 500x500px (auto-resized)
- Max upload: 2MB

### Preferences Storage
- JSONB column: flexible, can add new preferences without migration
- Indexed columns: `timezone`, `theme` for fast queries
- Merge logic: partial updates preserve existing preferences

### Security
- All endpoints require authentication
- Password change requires current password
- Avatar upload validates file type and size
- File paths sanitized to prevent directory traversal

---

Last Updated: 2025-11-12
Status: Backend Complete ✅ | Frontend In Progress 🚧
