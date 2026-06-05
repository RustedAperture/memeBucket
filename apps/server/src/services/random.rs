use rand::seq::IndexedRandom;
use uuid::Uuid;

use crate::repositories::{
    pools::PoolRepository,
    images::{ImageRepository, StoredImage},
    send_history::SendHistoryRepository,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RandomVisibility {
    Public,
    Private,
}

impl RandomVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            RandomVisibility::Public => "public",
            RandomVisibility::Private => "private",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RandomSelection {
    pub pool_name: String,
    pub url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RandomError {
    #[error("I could not find that pool.")]
    MissingPool,
    #[error("That pool has no saved images yet.")]
    EmptyPool,
    #[error("I hit a storage error while selecting media.")]
    Storage(#[source] sqlx::Error),
}

impl RandomError {
    pub fn user_message(&self) -> &'static str {
        match self {
            RandomError::MissingPool => "I could not find that pool.",
            RandomError::EmptyPool => "That pool has no saved images yet.",
            RandomError::Storage(_) => "I hit a storage error while selecting media.",
        }
    }

    pub fn is_private(&self) -> bool {
        true
    }
}

#[derive(Clone)]
pub struct RandomService {
    pools: PoolRepository,
    images: ImageRepository,
    history: SendHistoryRepository,
}

impl RandomService {
    pub fn new(
        pools: PoolRepository,
        images: ImageRepository,
        history: SendHistoryRepository,
    ) -> Self {
        Self {
            pools,
            images,
            history,
        }
    }

    pub async fn select_random(
        &self,
        owner_user_id: Uuid,
        pool_name: &str,
        visibility: RandomVisibility,
    ) -> Result<RandomSelection, RandomError> {
        let pool = self
            .pools
            .find_by_name_folded(owner_user_id, pool_name)
            .await
            .map_err(RandomError::Storage)?
            .ok_or(RandomError::MissingPool)?;

        let images = self
            .images
            .list_for_pool(owner_user_id, pool.id)
            .await
            .map_err(RandomError::Storage)?;

        let selected = choose_image(&images).ok_or(RandomError::EmptyPool)?;

        self.history
            .record(owner_user_id, &pool, selected, visibility.as_str())
            .await
            .map_err(RandomError::Storage)?;

        Ok(RandomSelection {
            pool_name: pool.name,
            url: selected.url.clone(),
        })
    }
}

fn choose_image(images: &[StoredImage]) -> Option<&StoredImage> {
    let mut rng = rand::rng();
    images.choose(&mut rng)
}
