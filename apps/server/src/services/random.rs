use rand::seq::IndexedRandom;
use uuid::Uuid;

use crate::repositories::{
    categories::CategoryRepository,
    media_links::{MediaLinkRepository, StoredMediaLink},
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
    pub category_name: String,
    pub url: String,
}

#[derive(Debug, thiserror::Error)]
pub enum RandomError {
    #[error("I could not find that category.")]
    MissingCategory,
    #[error("That category has no saved links yet.")]
    EmptyCategory,
    #[error("I hit a storage error while selecting media.")]
    Storage(#[source] sqlx::Error),
}

impl RandomError {
    pub fn user_message(&self) -> &'static str {
        match self {
            RandomError::MissingCategory => "I could not find that category.",
            RandomError::EmptyCategory => "That category has no saved links yet.",
            RandomError::Storage(_) => "I hit a storage error while selecting media.",
        }
    }

    pub fn is_private(&self) -> bool {
        true
    }
}

#[derive(Clone)]
pub struct RandomService {
    categories: CategoryRepository,
    links: MediaLinkRepository,
    history: SendHistoryRepository,
}

impl RandomService {
    pub fn new(
        categories: CategoryRepository,
        links: MediaLinkRepository,
        history: SendHistoryRepository,
    ) -> Self {
        Self {
            categories,
            links,
            history,
        }
    }

    pub async fn select_random(
        &self,
        owner_user_id: Uuid,
        category_name: &str,
        visibility: RandomVisibility,
    ) -> Result<RandomSelection, RandomError> {
        let category = self
            .categories
            .find_by_name_folded(owner_user_id, category_name)
            .await
            .map_err(RandomError::Storage)?
            .ok_or(RandomError::MissingCategory)?;

        let links = self
            .links
            .list_for_category(owner_user_id, category.id)
            .await
            .map_err(RandomError::Storage)?;

        let selected = choose_link(&links).ok_or(RandomError::EmptyCategory)?;

        self.history
            .record(owner_user_id, &category, selected, visibility.as_str())
            .await
            .map_err(RandomError::Storage)?;

        Ok(RandomSelection {
            category_name: category.name,
            url: selected.url.clone(),
        })
    }
}

fn choose_link(links: &[StoredMediaLink]) -> Option<&StoredMediaLink> {
    let mut rng = rand::rng();
    links.choose(&mut rng)
}
