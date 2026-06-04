use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}
