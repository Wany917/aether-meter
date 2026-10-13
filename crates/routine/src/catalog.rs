//! The community catalog (`catalog/routine.json`): what there is to do, how
//! often, and how banked counters recharge. Validated here with the same rules
//! as `catalog/routine.schema.json`, so a broken update is never applied.

use std::collections::{BTreeMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::bosses::BossDef;
use crate::error::CatalogError;
use crate::schedule::Cadence;

/// The catalog shipped with this build.
pub const BUNDLED: &str = include_str!("../../../catalog/routine.json");

/// Language code -> text. English is always present.
pub type Localized = BTreeMap<String, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "camelCase")]
pub enum TaskGroup {
    Daily,
    Weekly,
    Shop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub revision: u32,
    pub updated: String,
    pub reset_hour_utc: u8,
    pub tasks: Vec<TaskDef>,
    pub reserves: Vec<ReserveDef>,
    /// Since revision 3; older catalogs have none.
    #[serde(default)]
    pub bosses: Vec<BossDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDef {
    pub id: String,
    pub group: TaskGroup,
    pub cadence: Cadence,
    pub target: u16,
    #[serde(default)]
    pub membership_only: bool,
    pub label: Localized,
    #[serde(default)]
    pub note: Option<Localized>,
    #[serde(default)]
    pub guide: Option<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReserveDef {
    pub id: String,
    pub label: Localized,
    #[serde(default)]
    pub note: Option<Localized>,
    pub regen: u32,
    pub every_hours: u16,
    pub cap: u32,
    #[serde(default)]
    pub member_cap: Option<u32>,
    /// What one use adds or takes (a cube: 40 Odyle Energy). 1 if absent.
    #[serde(default)]
    pub step: Option<u32>,
    #[serde(default)]
    pub guide: Option<String>,
    #[serde(default)]
    pub sources: Vec<String>,
    pub verified: bool,
}

impl ReserveDef {
    pub fn cap_for(&self, membership: bool) -> u32 {
        if membership {
            self.member_cap.unwrap_or(self.cap)
        } else {
            self.cap
        }
    }
}

impl Catalog {
    pub fn bundled() -> Self {
        Self::parse(BUNDLED).expect("the bundled catalog is validated by the tests")
    }

    pub fn parse(json: &str) -> Result<Self, CatalogError> {
        let catalog: Catalog =
            serde_json::from_str(json).map_err(|e| CatalogError::Format(e.to_string()))?;
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn task(&self, id: &str) -> Option<&TaskDef> {
        self.tasks.iter().find(|t| t.id == id)
    }

    pub fn reserve(&self, id: &str) -> Option<&ReserveDef> {
        self.reserves.iter().find(|r| r.id == id)
    }

    fn validate(&self) -> Result<(), CatalogError> {
        let invalid = |id: &str, why: &str| Err(CatalogError::Invalid(format!("`{id}`: {why}")));
        if self.revision == 0 {
            return invalid("revision", "must be at least 1");
        }
        if self.reset_hour_utc > 23 {
            return invalid("resetHourUtc", "must be 0 to 23");
        }
        let mut ids = HashSet::new();
        let entries = self
            .tasks
            .iter()
            .map(|t| {
                (
                    &t.id,
                    [Some(&t.label), t.note.as_ref(), None],
                    &t.guide,
                    &t.sources,
                )
            })
            .chain(self.reserves.iter().map(|r| {
                (
                    &r.id,
                    [Some(&r.label), r.note.as_ref(), None],
                    &r.guide,
                    &r.sources,
                )
            }))
            .chain(self.bosses.iter().map(|b| {
                (
                    &b.id,
                    [Some(&b.label), b.note.as_ref(), b.zone.as_ref()],
                    &b.guide,
                    &b.sources,
                )
            }));
        for (id, texts, guide, sources) in entries {
            if !is_id(id) {
                return invalid(id, "ids are lowercase words joined by `-`");
            }
            if !ids.insert(id.as_str()) {
                return invalid(id, "duplicate id");
            }
            for text in texts.into_iter().flatten() {
                if !text.contains_key("en")
                    || text.values().any(|v| v.trim().is_empty() || v.len() > 200)
                {
                    return invalid(id, "texts need an `en` entry and 1 to 200 characters each");
                }
            }
            if guide
                .iter()
                .chain(sources)
                .any(|url| !url.starts_with("https://"))
            {
                return invalid(id, "links must be https");
            }
        }
        for task in &self.tasks {
            if !(1..=999).contains(&task.target) {
                return invalid(&task.id, "target must be 1 to 999");
            }
        }
        for reserve in &self.reserves {
            if reserve.regen == 0 || reserve.cap == 0 || !(1..=168).contains(&reserve.every_hours) {
                return invalid(
                    &reserve.id,
                    "regen and cap must be positive, everyHours 1 to 168",
                );
            }
            if reserve.member_cap.is_some_and(|m| m < reserve.cap) {
                return invalid(&reserve.id, "memberCap cannot be below cap");
            }
            if reserve.step == Some(0) {
                return invalid(&reserve.id, "step must be positive");
            }
        }
        let mut codes = HashSet::new();
        for boss in &self.bosses {
            if let Err(why) = boss.validate() {
                return invalid(&boss.id, &why);
            }
            if !boss.npc_codes.iter().all(|c| codes.insert(*c)) {
                return invalid(&boss.id, "an NPC code belongs to one boss only");
            }
        }
        Ok(())
    }
}

fn is_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 48
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

/// The text in `language`, else English.
pub fn localize(text: &Localized, language: &str) -> String {
    text.get(language)
        .or_else(|| text.get("en"))
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_catalog_is_valid() {
        let catalog = Catalog::parse(BUNDLED).unwrap();
        assert!(catalog.reserve("mission-quests").is_some());
        assert_eq!(catalog.reserve("odyle-energy").unwrap().step, Some(40));
        assert_eq!(catalog.reserve("odyle-energy").unwrap().cap_for(true), 840);
    }

    #[test]
    fn broken_catalogs_are_rejected() {
        let base: serde_json::Value = serde_json::from_str(BUNDLED).unwrap();
        let broken = |edit: fn(&mut serde_json::Value)| {
            let mut value = base.clone();
            edit(&mut value);
            Catalog::parse(&value.to_string())
        };
        assert!(
            broken(|v| v["tasks"][1]["id"] = "mission-quests".into()).is_err(),
            "duplicate id"
        );
        assert!(
            broken(|v| v["tasks"][0]["target"] = 0.into()).is_err(),
            "zero target"
        );
        assert!(
            broken(|v| v["reserves"][0]["memberCap"] = 10.into()).is_err(),
            "member cap below cap"
        );
        assert!(
            broken(|v| v["tasks"][0]["guide"] = "http://x".into()).is_err(),
            "plain http"
        );
        assert!(
            broken(|v| v["tasks"][0]["label"] = serde_json::json!({ "fr": "x" })).is_err(),
            "no english"
        );
        assert!(
            broken(|v| v["tasks"][0]["id"] = "Mission Quests".into()).is_err(),
            "bad id"
        );
    }

    #[test]
    fn missing_languages_fall_back_to_english() {
        let text = Localized::from([("en".to_string(), "Hello".to_string())]);
        assert_eq!(localize(&text, "de"), "Hello");
    }
}
