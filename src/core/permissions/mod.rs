use serenity::model::id::UserId;
use std::sync::OnceLock;

static BOT_OWNER_ID: OnceLock<Option<UserId>> = OnceLock::new();

pub fn init_bot_owner_id(id: Option<UserId>) {
    let _ = BOT_OWNER_ID.set(id);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLevel {
    Owner,
    Admin,
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

pub fn get_permission_level(
    guild_owner_id: UserId,
    user_id: UserId,
    has_administrator: bool,
) -> PermissionLevel {
    if BOT_OWNER_ID
        .get()
        .and_then(|o| *o)
        .map_or(false, |bot_owner| bot_owner == user_id)
    {
        return PermissionLevel::Owner;
    }
    if guild_owner_id == user_id || has_administrator {
        return PermissionLevel::Admin;
    }
    PermissionLevel::Member
}

#[inline]
pub fn has_permission(user_level: PermissionLevel, required: PermissionLevel) -> bool {
    user_level >= required
}

pub fn required_permission_message(required: PermissionLevel) -> &'static str {
    match required {
        PermissionLevel::Owner => "bot owner",
        PermissionLevel::Admin => "server admin",
        PermissionLevel::Member => "member",
    }
}
