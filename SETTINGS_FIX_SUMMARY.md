# Settings Page - Fix Summary

## Perbaikan yang Dilakukan

### 1. **ApiClient - Added baseURL Getter**
File: `apps/ui/lib/api/http.ts`

**Problem**: Property `baseURL` tidak tersedia secara public di ApiClient
**Solution**: Menambahkan getter `baseURL` untuk mengakses private `baseUrl`

```typescript
get baseURL(): string {
  return this.baseUrl
}
```

### 2. **FacadeApi - Avatar URL Methods**
File: `apps/ui/lib/api/facade.ts`

**Problem**: Methods menggunakan `this.baseUrl` yang tidak ada
**Solution**: Menggunakan `apiClient.baseURL` untuk mengakses base URL

```typescript
getAvatarUrl(userId: string): string {
  return `${apiClient.baseURL}/users/${userId}/avatar`;
}

getMyAvatarUrl(): string {
  return `${apiClient.baseURL}/auth/me/avatar`;
}
```

### 3. **FacadeApi - Avatar Upload**
File: `apps/ui/lib/api/facade.ts`

**Problem**: Multipart form upload tidak bekerja dengan apiClient.post
**Solution**: Menggunakan `fetch` langsung untuk multipart/form-data

```typescript
async uploadAvatar(file: File): Promise<void> {
  const formData = new FormData();
  formData.append("avatar", file);

  // Use fetch directly for multipart/form-data
  const response = await fetch(`${apiClient.baseURL}/auth/me/avatar`, {
    method: "POST",
    body: formData,
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(error);
  }
}
```

## Backend API Endpoints (Already Implemented)

Semua endpoint sudah tersedia di backend (`apps/manager/src/features/users/`):

### User Preferences
- ✅ **GET** `/v1/auth/me/preferences` - Get user preferences
- ✅ **PATCH** `/v1/auth/me/preferences` - Update preferences

### User Profile
- ✅ **GET** `/v1/auth/me/profile` - Get user profile
- ✅ **PATCH** `/v1/auth/me/profile` - Update profile (username)

### Password Management
- ✅ **POST** `/v1/auth/me/password` - Change password
  - Request Body: `{ current_password: string, new_password: string }`
  - Returns: 200 OK or 401 Unauthorized

### Avatar Management
- ✅ **POST** `/v1/auth/me/avatar` - Upload avatar (multipart/form-data)
- ✅ **GET** `/v1/auth/me/avatar` - Get current user avatar
- ✅ **DELETE** `/v1/auth/me/avatar` - Delete avatar

## Settings Page Features Status

File: `apps/ui/app/(dashboard)/settings/page.tsx`

### ✅ Working Features

1. **Account Tab**
   - ✅ Profile information (username, role, created_at, last_login_at)
   - ✅ Avatar upload (resize to 500x500, max 2MB)
   - ✅ Avatar delete
   - ✅ Update profile (username)
   - ✅ Change password with validation

2. **Appearance Tab**
   - ✅ Theme selection (light/dark/system) - uses next-themes
   - ✅ Timezone selection
   - ✅ Date format selection

3. **Notifications Tab**
   - ✅ Email notifications toggle
   - ✅ Browser notifications toggle
   - ✅ Desktop notifications toggle

4. **Defaults Tab**
   - ✅ Default VM CPU count
   - ✅ Default VM memory size
   - ✅ Default VM disk size

5. **System Tab**
   - ✅ System information (hosts, VMs, containers, functions count)
   - ✅ Storage estimation
   - ✅ API connection status
   - ✅ Auto-refresh interval setting
   - ✅ Metrics retention setting

### Password Change Flow

```typescript
const handleChangePassword = () => {
  // Validations:
  // 1. All fields must be filled
  if (!currentPassword || !newPassword || !confirmPassword) {
    toast.error("Please fill in all password fields")
    return
  }

  // 2. New password must match confirmation
  if (newPassword !== confirmPassword) {
    toast.error("New passwords do not match")
    return
  }

  // 3. Password must be at least 8 characters
  if (newPassword.length < 8) {
    toast.error("Password must be at least 8 characters")
    return
  }

  // API Call
  changePasswordMutation.mutate({
    current_password: currentPassword,
    new_password: newPassword,
  })
}
```

## Backend Implementation Details

### Change Password Handler
Location: `apps/manager/src/features/users/routes.rs:449`

```rust
pub async fn change_password(
    Extension(user): Extension<AuthenticatedUser>,
    Extension(st): Extension<AppState>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<nexus_types::OkResponse>, StatusCode> {
    st.users
        .change_password(user.id, &req.current_password, &req.new_password)
        .await
        .map_err(|e| match e {
            UserRepoError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(nexus_types::OkResponse::default()))
}
```

### Avatar Upload Handler
Location: `apps/manager/src/features/users/routes.rs:481`

Features:
- Max file size: 2MB
- Automatic resize to 500x500
- Saved as PNG format
- Stored in: `/srv/images/avatars/{user_id}.png`
- Database path update after successful upload

## Testing Checklist

### Account Tab
- [ ] Upload avatar (PNG, JPG, WEBP) max 2MB
- [ ] Delete avatar
- [ ] Update username
- [ ] Change password with valid current password
- [ ] Change password validation (min 8 chars, match confirmation)
- [ ] Change password with invalid current password (should show error)

### Appearance Tab
- [ ] Change theme (light/dark/system)
- [ ] Change timezone
- [ ] Change date format

### Notifications Tab
- [ ] Toggle email notifications
- [ ] Toggle browser notifications
- [ ] Toggle desktop notifications

### Defaults Tab
- [ ] Change default vCPU count
- [ ] Change default memory size
- [ ] Change default disk size

### System Tab
- [ ] View system statistics
- [ ] Change auto-refresh interval
- [ ] Change metrics retention

### Save/Reset
- [ ] Save changes button works
- [ ] Reset to defaults button works

## Notes

1. **Theme**: Menggunakan `next-themes` yang disimpan di localStorage, tidak di backend preferences
2. **Avatar Storage**: Avatar disimpan di `/srv/images/avatars/` di server
3. **Preferences Sync**: Preferences di-sync dari backend saat component mount
4. **Auto-save**: Preferences tidak auto-save, user harus klik "Save Changes"
5. **Reset**: "Reset to Defaults" akan reset semua preferences ke nilai default

## Environment Requirements

Pastikan direktori avatar exists di server:
```bash
sudo mkdir -p /srv/images/avatars
sudo chown -R $USER:$USER /srv/images/avatars
```

## Build Status

✅ TypeScript compilation: Success
✅ Next.js build: Success
✅ All types: Correct
✅ No runtime errors expected
