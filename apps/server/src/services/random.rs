use rand::Rng;
use uuid::Uuid;

use crate::repositories::{
    images::{ImageRepository, StoredImage},
    pools::{PoolRepository, StoredPool},
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
    #[error("That pool has no random-enabled images yet.")]
    NoRandomEnabledImages,
    #[error("I hit a storage error while selecting media.")]
    Storage(#[source] sqlx::Error),
}

impl RandomError {
    pub fn user_message(&self) -> &'static str {
        match self {
            RandomError::MissingPool => "I could not find that pool.",
            RandomError::EmptyPool => "That pool has no saved images yet.",
            RandomError::NoRandomEnabledImages => "That pool has no random-enabled images yet.",
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

#[derive(Clone, Debug)]
struct WeightedChoice {
    pool: StoredPool,
    image: StoredImage,
    weight: u32,
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
        requester_user_id: Uuid,
        pool_name: &str,
        visibility: RandomVisibility,
    ) -> Result<RandomSelection, RandomError> {
        self.select_random_from_pools(requester_user_id, &[pool_name], visibility)
            .await
    }

    pub async fn select_random_from_pools(
        &self,
        requester_user_id: Uuid,
        pool_names: &[&str],
        visibility: RandomVisibility,
    ) -> Result<RandomSelection, RandomError> {
        let mut all_choices = Vec::new();
        let mut eligible_choices = Vec::new();
        let mut pool_ids = Vec::new();

        for pool_name in pool_names {
            let pool = self
                .pools
                .find_accessible_by_name_folded(requester_user_id, pool_name)
                .await
                .map_err(RandomError::Storage)?
                .ok_or(RandomError::MissingPool)?;
            pool_ids.push(pool.id);

            let images = self
                .images
                .list_for_pool(requester_user_id, pool.id)
                .await
                .map_err(RandomError::Storage)?;

            all_choices.extend(images.iter().cloned().map(|image| WeightedChoice {
                pool: pool.clone(),
                weight: effective_weight(&image),
                image,
            }));
            eligible_choices.extend(images.into_iter().filter_map(|image| {
                let weight = effective_weight(&image);
                (weight > 0).then(|| WeightedChoice {
                    pool: pool.clone(),
                    image,
                    weight,
                })
            }));
        }

        if all_choices.is_empty() {
            return Err(RandomError::EmptyPool);
        }

        if eligible_choices.is_empty() {
            return Err(RandomError::NoRandomEnabledImages);
        }

        let recent_limit = eligible_choices.len().saturating_sub(1).min(5);
        let recent_image_ids = self
            .history
            .recent_image_ids_for_pools(requester_user_id, &pool_ids, recent_limit)
            .await
            .map_err(RandomError::Storage)?;
        let candidate_choices = exclude_recent_if_possible(eligible_choices, &recent_image_ids);

        let selected =
            choose_weighted_image(&candidate_choices).ok_or(RandomError::NoRandomEnabledImages)?;

        self.history
            .record(
                requester_user_id,
                &selected.pool,
                &selected.image,
                visibility.as_str(),
            )
            .await
            .map_err(RandomError::Storage)?;

        Ok(RandomSelection {
            pool_name: selected.pool.name.clone(),
            url: selected.image.url.clone(),
        })
    }
}

fn effective_weight(image: &StoredImage) -> u32 {
    let clamped = image.random_weight.clamp(0, 10);
    if image.favorite && clamped == 1 {
        3
    } else {
        clamped as u32
    }
}

fn exclude_recent_if_possible(
    choices: Vec<WeightedChoice>,
    recent_image_ids: &[Uuid],
) -> Vec<WeightedChoice> {
    if recent_image_ids.is_empty() {
        return choices;
    }

    let filtered: Vec<_> = choices
        .iter()
        .filter(|choice| !recent_image_ids.contains(&choice.image.id))
        .cloned()
        .collect();

    if filtered.is_empty() {
        choices
    } else {
        filtered
    }
}

fn choose_weighted_image(choices: &[WeightedChoice]) -> Option<&WeightedChoice> {
    let total_weight = choices.iter().fold(0_u64, |sum, choice| {
        sum.saturating_add(u64::from(choice.weight))
    });
    if total_weight == 0 {
        return None;
    }

    let mut rng = rand::rng();
    let mut pick = rng.random_range(0..total_weight);

    for choice in choices {
        let choice_weight = u64::from(choice.weight);
        if pick < choice_weight {
            return Some(choice);
        }
        pick -= choice_weight;
    }

    choices.last()
}

#[cfg(test)]
mod tests {
    use super::{WeightedChoice, effective_weight, exclude_recent_if_possible};
    use crate::repositories::{images::StoredImage, pools::StoredPool};
    use uuid::Uuid;

    #[test]
    fn favorite_default_weight_gets_boost() {
        let favorite = stored_image(true, 1);
        let explicit = stored_image(true, 4);
        let zero = stored_image(true, 0);

        assert_eq!(effective_weight(&favorite), 3);
        assert_eq!(effective_weight(&explicit), 4);
        assert_eq!(effective_weight(&zero), 0);
    }

    #[test]
    fn effective_weight_clamps_absurd_values() {
        let huge = stored_image(false, i64::MAX);
        let negative = stored_image(false, -7);
        let favorite_huge = stored_image(true, i64::MAX);

        assert_eq!(effective_weight(&huge), 10);
        assert_eq!(effective_weight(&negative), 0);
        assert_eq!(effective_weight(&favorite_huge), 10);
    }

    #[test]
    fn recent_filter_relaxes_when_it_would_remove_everything() {
        let image_id = Uuid::new_v4();
        let choices = vec![weighted_choice(image_id, 1)];

        let filtered = exclude_recent_if_possible(choices.clone(), &[image_id]);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].image.id, choices[0].image.id);
    }

    fn weighted_choice(image_id: Uuid, weight: u32) -> WeightedChoice {
        WeightedChoice {
            pool: StoredPool {
                id: Uuid::new_v4(),
                owner_user_id: Uuid::new_v4(),
                name: "cats".to_string(),
                share_token: None,
                subscriber_count: 0,
                owner_username: None,
                whitelist_enabled: false,
            },
            image: StoredImage {
                id: image_id,
                owner_user_id: Uuid::new_v4(),
                pool_id: Uuid::new_v4(),
                url: "https://example.com/cat.gif".to_string(),
                title: None,
                favorite: false,
                random_weight: i64::from(weight),
                tags: Vec::new(),
                created_at: String::new(),
                notes: None,
            },
            weight,
        }
    }

    fn stored_image(favorite: bool, random_weight: i64) -> StoredImage {
        StoredImage {
            id: Uuid::new_v4(),
            owner_user_id: Uuid::new_v4(),
            pool_id: Uuid::new_v4(),
            url: "https://example.com/cat.gif".to_string(),
            title: None,
            favorite,
            random_weight,
            tags: Vec::new(),
            created_at: String::new(),
            notes: None,
        }
    }
}
