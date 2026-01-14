+++
title = "Users"
description = "Complete guide to user management and access control"
weight = 80
date = 2025-01-08
+++

User Management allows you to create and manage user accounts, assign roles, and control access to your platform. This guide covers user creation, role-based access control, and account management through the web interface.

---

## What is User Management?

User Management is the **central system for controlling access** to your platform. It allows administrators to create user accounts, assign roles, and manage permissions for all users.

### Key Benefits

**1. Role-Based Access Control**
- Three user roles: Admin, User, and Viewer
- Control what each user can see and do
- Protect sensitive operations

**2. Centralized User Administration**
- View all users in one place
- Create, edit, and delete users easily
- Track user activity and login history

**3. Self-Service Account Management**
- Users can update their own profile
- Change password functionality
- Upload profile avatars

---

## User Roles

### Admin Role

**Full platform access**:
- Create, edit, and delete users
- Access all VMs, networks, volumes
- Manage all system settings
- View all resources across the platform

**Best for**:
- System administrators
- Team leads
- IT managers

---

### User Role

**Standard operational access**:
- Create and manage own VMs
- Access assigned resources
- Cannot manage other users
- Standard day-to-day operations

**Best for**:
- Developers
- Operations team members
- Regular platform users

---

### Viewer Role

**Read-only access**:
- View VMs, networks, volumes
- Cannot create or modify resources
- No write permissions
- Monitoring and observation only

**Best for**:
- Auditors
- Stakeholders who need visibility
- New team members in training

---

## User Properties

Each user account includes:

**Basic Information**:
- **Username** - Unique identifier for login
- **Role** - Access level (Admin, User, Viewer)
- **Password** - Secure authentication credential

**Profile Information**:
- **Avatar** - Profile picture (optional)
- **Timezone** - User's preferred timezone
- **Theme** - Dark or light mode preference

**Activity Tracking**:
- **Created At** - When the account was created
- **Last Login** - Most recent login timestamp

---

## User Lifecycle

### 1. Account Creation

**How users are created**:
1. Admin navigates to Users page
2. Clicks "Create User" button
3. Fills in username, password, and role
4. User account is created
5. User can log in immediately

---

### 2. Account Usage

**During active use**:
- User logs in with credentials
- System tracks login activity
- User accesses resources based on role
- User can update own profile

---

### 3. Account Management

**Ongoing administration**:
- Admin can edit user details
- Role can be changed as needed
- Password can be reset
- Account can be deleted when no longer needed

---

## Quick Start

### 1. Navigate to Users Page

![Image: Users navigation](/images/users/nav-users.png)

Click **"Users"** in the sidebar to access User Management.

---

### 2. View User List

![Image: Users page](/images/users/page-layout.png)

The Users page shows:
- Total user count
- Number of admins
- User table with all accounts
- Search and filter options

---

### 3. Create a New User

![Image: Create user](/images/users/create-user-button.png)

1. Click **"Create User"** button
2. Fill in the user details:
   - Username (required)
   - Password (required)
   - Role (Admin, User, or Viewer)
3. Click **"Create"**

---

### 4. Manage Existing Users

![Image: User actions](/images/users/user-actions.png)

For each user, you can:
- **Edit** - Update username, password, or role
- **Delete** - Remove the user account

---

## Common Use Cases

### Team Onboarding

**Add new team members**:
1. Go to Users page
2. Create user with appropriate role
3. Share credentials securely
4. User logs in and starts working

**Example**:
```
New Developer:
- Username: john.developer
- Role: User
- Access: Can create and manage VMs
```

---

### Role Adjustment

**Change user permissions**:
1. Find user in the table
2. Click Edit button
3. Change role as needed
4. Save changes

**Example**:
```
Promoting to Admin:
- User: jane.ops
- Old Role: User
- New Role: Admin
- Result: Full platform access
```

---

### Offboarding

**Remove departing team members**:
1. Find user in the table
2. Click Delete button
3. Confirm deletion
4. Account is removed

**Important**: You cannot delete your own account. Another admin must do this.

---

### Access Auditing

**Review who has access**:
1. Go to Users page
2. Use role filter to see all Admins
3. Review last login dates
4. Identify inactive accounts

---

## Security Best Practices

### 1. Use Strong Passwords

**Password guidelines**:
- Minimum 8 characters
- Mix of letters, numbers, symbols
- Avoid common words
- Unique for each user

---

### 2. Principle of Least Privilege

**Assign minimum required access**:
- New users start as Viewer
- Promote to User when needed
- Admin only for administrators
- Regular access reviews

---

### 3. Regular Access Reviews

**Periodic audits**:
- Review user list monthly
- Remove inactive accounts
- Verify role assignments
- Check last login dates

---

### 4. Protect Admin Accounts

**Admin account security**:
- Limit number of admins
- Strong passwords required
- Monitor admin activity
- Remove admin access when not needed

---

## User Interface Overview

### Users Page Header

![Image: Users header](/images/users/header.png)

**Displays**:
- "User Management" title
- Total Users count
- Admins count
- Visual icon

---

### User Table

![Image: User table](/images/users/table.png)

**Columns**:
- **Username** - User's login name (shows "You" badge for current user)
- **Role** - Color-coded role badge
- **Created** - Account creation date
- **Last Login** - Most recent login (or "Never")
- **Actions** - Edit and Delete buttons

---

### Filters

![Image: Filters](/images/users/filters.png)

**Available filters**:
- **Search** - Find users by username
- **Role** - Filter by Admin, User, or Viewer

---

## Role Permissions Matrix

| Feature | Admin | User | Viewer |
|---------|-------|------|--------|
| View Users | Yes | No | No |
| Create Users | Yes | No | No |
| Edit Users | Yes | No | No |
| Delete Users | Yes | No | No |
| View VMs | Yes | Yes | Yes |
| Create VMs | Yes | Yes | No |
| Manage VMs | Yes | Own | No |
| View Networks | Yes | Yes | Yes |
| Manage Networks | Yes | Yes | No |
| View Volumes | Yes | Yes | Yes |
| Manage Volumes | Yes | Yes | No |
| System Settings | Yes | No | No |

---

## Troubleshooting

### Cannot Login

**Symptoms**:
- Login fails with credentials
- Error message appears

**Possible causes**:
1. Incorrect username
2. Wrong password
3. Account deleted

**Solution**:
1. Double-check username spelling
2. Try resetting password (contact admin)
3. Verify account exists in Users page

---

### Cannot Create User

**Symptoms**:
- Create user fails
- Error notification appears

**Possible causes**:
1. Username already exists
2. Required fields empty
3. Server connection issue

**Solution**:
1. Try a different username
2. Fill all required fields
3. Refresh page and try again

---

### Cannot Delete User

**Symptoms**:
- Delete button disabled
- Cannot remove user

**Possible causes**:
1. Trying to delete yourself
2. User has associated resources

**Solution**:
1. Ask another admin to delete (if deleting yourself)
2. Remove user's resources first

---

### Role Not Changing

**Symptoms**:
- Edit role but no change
- User still has old permissions

**Possible causes**:
1. Edit didn't save
2. Page not refreshed

**Solution**:
1. Ensure you clicked Save
2. Refresh the page
3. Verify change in user table

---

## Best Practices

### 1. Naming Convention

**Use consistent usernames**:
```
Format: firstname.lastname
Examples:
- john.smith
- jane.doe
- admin.main
```

**Benefits**:
- Easy to identify
- Professional appearance
- Consistent across platform

---

### 2. Document User Access

**Keep external records**:
```
User: john.developer
Role: User
Department: Engineering
Added: 2025-01-08
Purpose: VM development access
```

---

### 3. Regular Cleanup

**Maintain clean user list**:
- Remove departed employees
- Deactivate inactive accounts
- Review role assignments quarterly

---

### 4. Admin Redundancy

**Multiple admins recommended**:
- At least 2 admin accounts
- Don't rely on single admin
- Backup admin for emergencies

---

## Quick Reference

### User Actions

| Action | Steps | Who Can Do It |
|--------|-------|---------------|
| Create User | Users page → Create User → Fill form | Admin only |
| Edit User | Users page → Edit button → Update form | Admin only |
| Delete User | Users page → Delete button → Confirm | Admin only |
| Change Own Password | Profile → Change Password | All users |
| Update Own Profile | Profile → Edit Profile | All users |
| View Users | Navigate to Users page | Admin only |

---

### Role Badges

| Role | Badge Color | Description |
|------|-------------|-------------|
| Admin | Red | Full platform access |
| User | Blue | Standard operational access |
| Viewer | Gray | Read-only access |

---

## Next Steps

- **[Manage Users](manage-users/)** - Detailed guide to user management operations
- **[VMs](/docs/vm/)** - Create and manage virtual machines
- **[Networks](/docs/networks/)** - Configure network settings
- **[Volumes](/docs/volumes/)** - Manage storage volumes

---

## FAQ

**Q: How many admins should I have?**
A: We recommend at least 2 admin accounts. This ensures you're not locked out if one admin is unavailable. However, limit admin access to only those who truly need it.

**Q: Can I delete my own account?**
A: No. For security reasons, you cannot delete your own account. Another admin must delete your account if needed.

**Q: What happens when a user is deleted?**
A: The user account is permanently removed. Any resources they created (VMs, etc.) remain in the system. Consider reassigning resources before deleting.

**Q: Can I change a username?**
A: Yes. Admins can edit any user's username through the Edit function. The user will need to log in with the new username.

**Q: What if I forget my password?**
A: Contact an administrator. They can reset your password through the Edit user function. There is no self-service password reset currently.

**Q: Can users see each other's resources?**
A: This depends on the platform configuration. Generally, admins can see all resources, while users see their own resources. Viewers can see resources but cannot modify them.
