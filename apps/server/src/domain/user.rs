use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct User {
    pub id: Uuid,
    pub discord_user_key: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}
