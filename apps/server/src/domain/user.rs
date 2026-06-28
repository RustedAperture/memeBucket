use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UserRole {
    User,
    Admin,
}

impl UserRole {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "admin" => UserRole::Admin,
            _ => UserRole::User,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::User => "user",
            UserRole::Admin => "admin",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthProvider {
    Discord,
    Telegram,
}

impl AuthProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            AuthProvider::Discord => "discord",
            AuthProvider::Telegram => "telegram",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "discord" => Some(AuthProvider::Discord),
            "telegram" => Some(AuthProvider::Telegram),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct UserIdentity {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider: AuthProvider,
    pub provider_user_id: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}
