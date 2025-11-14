/// Authorization module for role-based access control with resource ownership
///
/// This module provides permission checking functions for the RBAC system.
/// It supports three roles (Admin, User, Viewer) with resource ownership checks.
use nexus_types::Role;
use uuid::Uuid;

/// Check if a user can create resources (VMs, functions, containers, etc.)
///
/// Permission matrix:
/// - Admin: ✅ Can create
/// - User: ✅ Can create
/// - Viewer: ❌ Cannot create (read-only)
#[allow(dead_code)]
pub fn can_create_resource(role: Role) -> bool {
    matches!(role, Role::Admin | Role::User)
}

/// Check if a user can view a specific resource
///
/// Permission matrix:
/// - Admin: ✅ Can view all resources
/// - User: ✅ Can view own resources + unowned resources
/// - Viewer: ✅ Can view all resources (read-only)
///
/// A resource with `owner_id = None` is considered "unowned" and viewable by all authenticated users.
#[allow(dead_code)]
pub fn can_view_resource(role: Role, owner_id: Option<Uuid>, user_id: Uuid) -> bool {
    match role {
        Role::Admin => true,  // Admins can view everything
        Role::Viewer => true, // Viewers can view everything (read-only)
        Role::User => {
            // Users can view their own resources or unowned resources
            owner_id.is_none() || owner_id == Some(user_id)
        }
    }
}

/// Check if a user can modify a specific resource (update/start/stop operations)
///
/// Permission matrix:
/// - Admin: ✅ Can modify all resources
/// - User: ✅ Can modify own resources only
/// - Viewer: ❌ Cannot modify (read-only)
///
/// A resource with `owner_id = None` can only be modified by admins.
#[allow(dead_code)]
pub fn can_modify_resource(role: Role, owner_id: Option<Uuid>, user_id: Uuid) -> bool {
    match role {
        Role::Admin => true,   // Admins can modify everything
        Role::Viewer => false, // Viewers cannot modify anything
        Role::User => {
            // Users can only modify resources they own
            owner_id == Some(user_id)
        }
    }
}

/// Check if a user can delete a specific resource
///
/// Permission matrix:
/// - Admin: ✅ Can delete all resources
/// - User: ✅ Can delete own resources only
/// - Viewer: ❌ Cannot delete (read-only)
///
/// A resource with `owner_id = None` can only be deleted by admins.
#[allow(dead_code)]
pub fn can_delete_resource(role: Role, owner_id: Option<Uuid>, user_id: Uuid) -> bool {
    // Same logic as modify - if you can modify it, you can delete it
    can_modify_resource(role, owner_id, user_id)
}

/// Check if a user can manage users (create/update/delete users)
///
/// Permission matrix:
/// - Admin: ✅ Can manage users
/// - User: ❌ Cannot manage users
/// - Viewer: ❌ Cannot manage users
#[allow(dead_code)]
pub fn can_manage_users(role: Role) -> bool {
    matches!(role, Role::Admin)
}

/// Check if a user can view audit logs
///
/// Permission matrix:
/// - Admin: ✅ Can view audit logs
/// - User: ❌ Cannot view audit logs
/// - Viewer: ❌ Cannot view audit logs
#[allow(dead_code)]
pub fn can_view_audit_logs(role: Role) -> bool {
    matches!(role, Role::Admin)
}

/// Check if a user can perform administrative actions on networks
///
/// Networks are special - they can affect multiple VMs, so only admins
/// and the creator can delete them. But users can create their own networks.
#[allow(dead_code)]
pub fn can_delete_network(role: Role, owner_id: Option<Uuid>, user_id: Uuid) -> bool {
    match role {
        Role::Admin => true,
        Role::User => owner_id == Some(user_id),
        Role::Viewer => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_create_resource() {
        assert!(can_create_resource(Role::Admin));
        assert!(can_create_resource(Role::User));
        assert!(!can_create_resource(Role::Viewer));
    }

    #[test]
    fn test_can_view_resource() {
        let admin_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();

        // Admin can view everything
        assert!(can_view_resource(Role::Admin, Some(user_id), admin_id));
        assert!(can_view_resource(Role::Admin, None, admin_id));

        // Viewer can view everything
        assert!(can_view_resource(
            Role::Viewer,
            Some(user_id),
            other_user_id
        ));
        assert!(can_view_resource(Role::Viewer, None, other_user_id));

        // User can view own resources
        assert!(can_view_resource(Role::User, Some(user_id), user_id));

        // User can view unowned resources
        assert!(can_view_resource(Role::User, None, user_id));

        // User cannot view other users' resources
        assert!(!can_view_resource(Role::User, Some(other_user_id), user_id));
    }

    #[test]
    fn test_can_modify_resource() {
        let admin_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let other_user_id = Uuid::new_v4();

        // Admin can modify everything
        assert!(can_modify_resource(Role::Admin, Some(user_id), admin_id));
        assert!(can_modify_resource(Role::Admin, None, admin_id));

        // Viewer cannot modify anything
        assert!(!can_modify_resource(Role::Viewer, Some(user_id), user_id));
        assert!(!can_modify_resource(Role::Viewer, None, user_id));

        // User can modify own resources
        assert!(can_modify_resource(Role::User, Some(user_id), user_id));

        // User cannot modify unowned resources
        assert!(!can_modify_resource(Role::User, None, user_id));

        // User cannot modify other users' resources
        assert!(!can_modify_resource(
            Role::User,
            Some(other_user_id),
            user_id
        ));
    }

    #[test]
    fn test_can_delete_resource() {
        // Same logic as modify
        let user_id = Uuid::new_v4();
        assert!(can_delete_resource(
            Role::Admin,
            Some(user_id),
            Uuid::new_v4()
        ));
        assert!(can_delete_resource(Role::User, Some(user_id), user_id));
        assert!(!can_delete_resource(Role::Viewer, Some(user_id), user_id));
    }

    #[test]
    fn test_can_manage_users() {
        assert!(can_manage_users(Role::Admin));
        assert!(!can_manage_users(Role::User));
        assert!(!can_manage_users(Role::Viewer));
    }

    #[test]
    fn test_can_view_audit_logs() {
        assert!(can_view_audit_logs(Role::Admin));
        assert!(!can_view_audit_logs(Role::User));
        assert!(!can_view_audit_logs(Role::Viewer));
    }
}
