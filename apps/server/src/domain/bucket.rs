use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bucket {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub name: String,
}
