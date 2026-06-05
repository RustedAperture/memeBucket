use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Image {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub pool_id: Uuid,
    pub url: String,
}
