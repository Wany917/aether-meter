//! Game data shipped next to the app: localized names and icon tables.
//!
//! The files come from the A2Tools repository at the pinned parser revision
//! (`bun run sync:gamedata`), so names always match what the parser decodes.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use a2tools::capture::abnormal;
use a2tools::i18n::lookup::{self, NpcLookup, SkillLookup};
use parking_lot::RwLock;
use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::error::EngineError;

/// Every locale the data files carry. Anything else falls back to English.
pub const LANGUAGES: [&str; 10] = [
    "de", "en", "es", "fr", "ja", "ko", "pt", "ru", "zh-Hans", "zh-Hant",
];

pub struct GameData {
    dir: PathBuf,
    pub skills: Arc<SkillLookup>,
    pub npcs: Arc<NpcLookup>,
    pub dot_skill_ids: HashSet<i32>,
    pub stack_limits: HashMap<u32, u32>,
    abnormal_icons: HashMap<u32, String>,
    /// First four digits of an 8-digit skill code -> icon file name.
    skill_icons: HashMap<String, String>,
    /// Server id -> name in each language.
    servers: HashMap<u16, HashMap<String, String>>,
    localized: RwLock<Localized>,
}

#[derive(Default)]
struct Localized {
    language: String,
    abnormals: HashMap<u32, String>,
    dungeons: HashMap<i32, Dungeon>,
    npcs: HashMap<i32, NpcEntry>,
    /// Instances whose bosses come in several levels, one code each (Nightmare).
    multi_tier_dungeons: HashSet<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NpcEntry {
    name: String,
    tier: Option<String>,
    dungeon_id: Option<i32>,
}

#[derive(Deserialize)]
struct Dungeon {
    name: String,
    difficulty: Option<String>,
}

#[derive(Deserialize)]
struct ServerTable {
    servers: HashMap<String, HashMap<String, String>>,
}

#[derive(Deserialize)]
struct AbnormalTable {
    abnormals: HashMap<String, AbnormalEntry>,
}

#[derive(Deserialize)]
struct AbnormalEntry {
    icon: Option<String>,
}

impl GameData {
    pub fn load(dir: &Path, language: &str) -> Result<Self, EngineError> {
        if !dir.join("i18n").join("skills").is_dir() {
            return Err(EngineError::GameDataMissing(dir.to_path_buf()));
        }

        let abnormal_table: Option<AbnormalTable> = read_json(&dir.join("abnormals.json"));
        let abnormal_icons = abnormal_table
            .map(|t| {
                t.abnormals
                    .into_iter()
                    .filter_map(|(id, entry)| Some((id.parse().ok()?, entry.icon?)))
                    .collect()
            })
            .unwrap_or_default();
        let stack_limits = std::fs::read_to_string(dir.join("abnormals.json"))
            .map(|text| abnormal::stack_limits(&text))
            .unwrap_or_default();

        let data = Self {
            dir: dir.to_path_buf(),
            skills: Arc::new(SkillLookup::new()),
            npcs: Arc::new(NpcLookup::new()),
            dot_skill_ids: read_json::<Vec<i32>>(&dir.join("dot_skill_ids.json"))
                .unwrap_or_default()
                .into_iter()
                .collect(),
            stack_limits,
            abnormal_icons,
            skill_icons: read_json(&dir.join("skill_icons.json")).unwrap_or_default(),
            servers: read_json::<ServerTable>(&dir.join("servers.json"))
                .map(|t| {
                    t.servers
                        .into_iter()
                        .filter_map(|(id, names)| Some((id.parse().ok()?, names)))
                        .collect()
                })
                .unwrap_or_default(),
            localized: RwLock::new(Localized::default()),
        };
        data.set_language(language);
        Ok(data)
    }

    /// A server's name in the current language ("Siel"), if the table has it.
    pub fn server_name(&self, id: u16) -> Option<String> {
        let names = self.servers.get(&id)?;
        names
            .get(&self.language())
            .or_else(|| names.get("en"))
            .cloned()
    }

    pub fn language(&self) -> String {
        self.localized.read().language.clone()
    }

    /// Reload every localized name. Skill and NPC lookups are swapped in place,
    /// so the running parser picks the new names up on its next snapshot.
    pub fn set_language(&self, language: &str) {
        let language = if LANGUAGES.contains(&language) {
            language
        } else {
            "en"
        };
        lookup::load_language(&self.skills, &self.npcs, &self.dir, language);

        let i18n = self.dir.join("i18n");
        let abnormals =
            read_localized::<HashMap<String, String>>(&i18n.join("abnormals"), language)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(id, name)| Some((id.parse().ok()?, name)))
                .collect();
        let dungeons = read_localized::<HashMap<String, Dungeon>>(&i18n.join("dungeons"), language)
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(id, d)| Some((id.parse().ok()?, d)))
            .collect();

        let npcs: HashMap<i32, NpcEntry> =
            read_localized::<HashMap<String, serde_json::Value>>(&i18n.join("npcs"), language)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|(id, value)| {
                    Some((id.parse().ok()?, serde_json::from_value(value).ok()?))
                })
                .collect();
        let mut tiers: HashMap<i32, HashSet<&str>> = HashMap::new();
        for npc in npcs.values() {
            if let (Some(dungeon), Some(tier)) = (npc.dungeon_id, npc.tier.as_deref()) {
                tiers.entry(dungeon).or_default().insert(tier);
            }
        }
        let multi_tier_dungeons = tiers
            .into_iter()
            .filter(|(_, t)| t.len() > 1)
            .map(|(d, _)| d)
            .collect();

        *self.localized.write() = Localized {
            language: language.to_string(),
            abnormals,
            dungeons,
            npcs,
            multi_tier_dungeons,
        };
    }

    /// "Gatekeeper Pinopi · Level 2": the NPC's name by code, with its level
    /// where its instance has several; `fallback` when the code is unknown.
    pub fn target_label(&self, mob_code: i32, fallback: &str) -> String {
        let localized = self.localized.read();
        let Some(npc) = localized
            .npcs
            .get(&mob_code)
            .filter(|n| !n.name.trim().is_empty())
        else {
            return fallback.trim().to_string();
        };
        match (npc.tier.as_deref(), npc.dungeon_id) {
            (Some(tier), Some(dungeon)) if localized.multi_tier_dungeons.contains(&dungeon) => {
                format!("{} · {tier}", npc.name)
            }
            _ => npc.name.clone(),
        }
    }

    pub fn skill_name(&self, code: i32) -> String {
        self.skills.lookup_skill_name(code)
    }

    pub fn abnormal_name(&self, id: u32) -> Option<String> {
        self.localized.read().abnormals.get(&id).cloned()
    }

    pub fn abnormal_icon(&self, id: u32) -> Option<&str> {
        self.abnormal_icons.get(&id).map(String::as_str)
    }

    pub fn skill_icon_table(&self) -> &HashMap<String, String> {
        &self.skill_icons
    }

    /// "Krao Cave · Normal", or `None` outside an instance.
    pub fn dungeon_label(&self, id: i32) -> Option<String> {
        if id <= 0 {
            return None;
        }
        let localized = self.localized.read();
        let dungeon = localized.dungeons.get(&id)?;
        Some(match dungeon.difficulty.as_deref() {
            Some(d) => format!(
                "{} · {}",
                dungeon.name,
                difficulty_label(d, &localized.language)
            ),
            None => dungeon.name.clone(),
        })
    }
}

/// Only English, Korean and Chinese ship dungeon tables; the difficulty words
/// are translated here for the languages that fall back to English names.
fn difficulty_label(difficulty: &str, language: &str) -> String {
    match (difficulty, language) {
        ("hard", "fr") => "Difficile".to_string(),
        ("hard", _) => "Hard".to_string(),
        ("normal", _) => "Normal".to_string(),
        ("exploration", _) => "Exploration".to_string(),
        (other, _) => other.to_string(),
    }
}

fn read_localized<T: DeserializeOwned>(dir: &Path, language: &str) -> Option<T> {
    read_json(&dir.join(format!("{language}.json"))).or_else(|| read_json(&dir.join("en.json")))
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(e) => {
            tracing::warn!("ignoring {}: {e}", path.display());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled(language: &str) -> GameData {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/overlay/src-tauri/resources/gamedata");
        GameData::load(&dir, language).expect("bundled game data")
    }

    #[test]
    fn nightmare_bosses_are_named_with_their_level() {
        let fr = bundled("fr");
        assert_eq!(
            fr.target_label(2_980_041, ""),
            "Pinopi le gardien des portes · Niveau 2"
        );
        let en = bundled("en");
        assert_eq!(
            en.target_label(2_980_049, ""),
            "Gatekeeper Pinopi · Level 10"
        );
    }

    #[test]
    fn unknown_codes_keep_the_parser_name() {
        let en = bundled("en");
        assert_eq!(en.target_label(0, " Some Boss "), "Some Boss");
        assert_eq!(en.target_label(1, ""), "");
    }

    #[test]
    fn servers_are_named_in_the_current_language() {
        assert_eq!(bundled("ko").server_name(1001).as_deref(), Some("시엘"));
        assert_eq!(bundled("fr").server_name(1001).as_deref(), Some("Siel"));
        assert_eq!(bundled("fr").server_name(0), None);
    }
}
