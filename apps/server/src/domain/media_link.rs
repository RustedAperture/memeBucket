use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MediaLink {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub category_id: Uuid,
    pub url: String,
}
