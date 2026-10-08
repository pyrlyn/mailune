//! Rank addresses that appeared in `From`: frequency times recency.
//!
//! Recency is how close the latest sighting is to `now`, measured inside a
//! 30-day horizon. `score = frequency * recency`. Someone seen just now has
//! recency `1 + horizon`. Someone last seen at the horizon or earlier has
//! recency `1`, so an old frequent address still sorts, under anyone recent.

use std::collections::BTreeMap;

/// One `From` address and when it was seen, in seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sighting {
    /// Address as it appeared. Grouping ignores ASCII case.
    pub email: String,
    /// Unix seconds.
    pub at_secs: u64,
}

/// One address after ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedContact {
    /// Spelling of the first sighting.
    pub email: String,
    /// How many sightings shared this address.
    pub frequency: u64,
    /// `1 +` remaining seconds inside the 30-day horizon.
    pub recency: u64,
    /// `frequency * recency`.
    pub score: u64,
}

const HORIZON_SECS: u64 = 30 * 24 * 60 * 60;

struct Bucket {
    email: String,
    count: u64,
    latest: u64,
}

/// Highest score first. Equal scores sort by the stored spelling.
pub fn rank_contacts(sightings: &[Sighting], now_secs: u64) -> Vec<RankedContact> {
    let mut groups: BTreeMap<String, Bucket> = BTreeMap::new();
    for sighting in sightings {
        let key = sighting.email.trim().to_ascii_lowercase();
        if key.is_empty() {
            continue;
        }
        match groups.entry(key) {
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let bucket = entry.get_mut();
                bucket.count += 1;
                bucket.latest = bucket.latest.max(sighting.at_secs);
            }
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(Bucket {
                    email: sighting.email.trim().to_string(),
                    count: 1,
                    latest: sighting.at_secs,
                });
            }
        }
    }

    let mut ranked: Vec<RankedContact> = groups
        .into_values()
        .map(|bucket| {
            let age = now_secs.saturating_sub(bucket.latest.min(now_secs));
            let recency = 1 + HORIZON_SECS.saturating_sub(age.min(HORIZON_SECS));
            RankedContact {
                email: bucket.email,
                frequency: bucket.count,
                score: bucket.count.saturating_mul(recency),
                recency,
            }
        })
        .collect();
    ranked.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.email.cmp(&right.email))
    });
    ranked
}

#[cfg(test)]
mod tests {
    use super::{Sighting, rank_contacts};

    #[test]
    fn frequency_times_recency_orders_from_addresses() {
        let now = 1_700_000_000;
        let day = 24 * 60 * 60;
        let sightings = [
            Sighting {
                email: "Ana@Acme.io".to_string(),
                at_secs: now - 3 * day,
            },
            Sighting {
                email: "ana@acme.io".to_string(),
                at_secs: now - day,
            },
            Sighting {
                email: "ana@acme.io".to_string(),
                at_secs: now,
            },
            Sighting {
                email: "bo@acme.io".to_string(),
                at_secs: now,
            },
            Sighting {
                email: "cy@acme.io".to_string(),
                at_secs: now - 40 * day,
            },
            Sighting {
                email: "cy@acme.io".to_string(),
                at_secs: now - 41 * day,
            },
            Sighting {
                email: "cy@acme.io".to_string(),
                at_secs: now - 42 * day,
            },
            Sighting {
                email: "cy@acme.io".to_string(),
                at_secs: now - 43 * day,
            },
        ];

        let ranked = rank_contacts(&sightings, now);
        assert_eq!(ranked.len(), 3);
        assert_eq!(ranked[0].email, "Ana@Acme.io");
        assert_eq!(ranked[0].frequency, 3);
        assert_eq!(ranked[1].email, "bo@acme.io");
        assert_eq!(ranked[1].frequency, 1);
        assert_eq!(ranked[2].email, "cy@acme.io");
        assert_eq!(ranked[2].frequency, 4);
        assert!(ranked[0].score > ranked[1].score);
        assert!(ranked[1].score > ranked[2].score);
        assert_eq!(ranked[0].score, ranked[0].frequency * ranked[0].recency);
        assert_eq!(ranked[2].recency, 1);
    }
}
