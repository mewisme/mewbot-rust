//! Permission hierarchy: Owner > Admin > Member.
//! - **Owner**: guild owner (from Discord).
//! - **Admin**: user has `Permissions::ADMINISTRATOR` (server admin / co-admin, same level).
//! - **Member**: everyone else.

use serenity::model::id::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLevel {
    /// Guild owner (highest).
    Owner,
    /// User has Administrator permission (server admin / co-admin; same level).
    Admin,
    /// Regular member (lowest).
    Member,
}

impl PartialOrd for PermissionLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PermissionLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self, other) {
            (PermissionLevel::Owner, PermissionLevel::Owner) => Ordering::Equal,
            (PermissionLevel::Owner, _) => Ordering::Greater,
            (PermissionLevel::Admin, PermissionLevel::Owner) => Ordering::Less,
            (PermissionLevel::Admin, PermissionLevel::Admin) => Ordering::Equal,
            (PermissionLevel::Admin, PermissionLevel::Member) => Ordering::Greater,
            (PermissionLevel::Member, PermissionLevel::Owner) => Ordering::Less,
            (PermissionLevel::Member, PermissionLevel::Admin) => Ordering::Less,
            (PermissionLevel::Member, PermissionLevel::Member) => Ordering::Equal,
        }
    }
}

/// Returns the user's permission level in the guild.
/// - If `owner_id == user_id` → Owner.
/// - Else if `has_administrator` (user has `Permissions::ADMINISTRATOR`) → Admin.
/// - Else → Member.
pub fn get_permission_level(
    owner_id: UserId,
    user_id: UserId,
    has_administrator: bool,
) -> PermissionLevel {
    if owner_id == user_id {
        return PermissionLevel::Owner;
    }
    if has_administrator {
        return PermissionLevel::Admin;
    }
    PermissionLevel::Member
}

/// Returns true if `user_level` is at least `required` (owner > admin > member).
#[inline]
pub fn has_permission(user_level: PermissionLevel, required: PermissionLevel) -> bool {
    user_level >= required
}
