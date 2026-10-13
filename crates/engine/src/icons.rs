//! Skill and buff icons on NCSOFT's public game-data CDN.
//!
//! Nothing is downloaded or redistributed here: the overlay loads these URLs
//! like any `<img>`, trying each candidate in order. Port of A2Tools'
//! `skillIcons.js` resolver (GPL-3.0).

use std::collections::HashMap;

const CDN: &str = "https://assets.playnccdn.com/static-aion2-gamedata/resources";
const THEOSTONE_ICON_BASE: &str = "Icon_Item_Usable_Godstone_WP_r_";

/// Candidate icon URLs for a skill, best first. Empty for basic attacks and
/// codes with no known icon: the overlay draws its own glyph for those.
pub fn skill_icons(code: i32, table: &HashMap<String, String>) -> Vec<String> {
    let Some(code) = normalize(code) else {
        return Vec::new();
    };

    if let Some(url) = theostone_icon(&code) {
        return vec![url];
    }
    if is_basic_attack(&code) {
        return Vec::new();
    }
    if let Some(name) = table.get(&code[..4]) {
        return vec![cdn(name)];
    }
    let Some(class) = class_code(&code[..2]) else {
        return Vec::new();
    };
    match code[2..4].parse::<u32>() {
        Ok(sub) => vec![cdn(&format!("ICON_{class}_SKILL_{sub:03}"))],
        Err(_) => Vec::new(),
    }
}

/// Candidate icon URLs for a buff: its own icon, the same without a
/// `_Buff`/`_Debuff` suffix (some of those are not on the CDN), then the icon
/// of the skill that applied it.
pub fn abnormal_icons(
    icon: Option<&str>,
    skill: Option<u32>,
    table: &HashMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(icon) = icon.filter(|i| i.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')) {
        out.push(cdn(icon));
        let plain = icon.trim_end_matches("_Buff").trim_end_matches("_Debuff");
        if plain != icon {
            out.push(cdn(plain));
        }
    }
    if let Some(code) = skill.and_then(|s| i32::try_from(s).ok()) {
        for url in skill_icons(code, table) {
            if !out.contains(&url) {
                out.push(url);
            }
        }
    }
    out
}

fn cdn(name: &str) -> String {
    format!("{CDN}/{name}.png")
}

/// Skill codes as an 8-digit string: longer codes are cut, shorter padded.
fn normalize(code: i32) -> Option<String> {
    if code <= 0 {
        return None;
    }
    let digits = code.to_string();
    Some(if digits.len() >= 8 {
        digits[..8].to_string()
    } else {
        format!("{digits:0<8}")
    })
}

fn is_basic_attack(code: &str) -> bool {
    let class_auto =
        ("11"..="18").contains(&&code[..2]) && &code[2..6] == "0000" && &code[6..] != "00";
    class_auto || code.starts_with("1000") || code.starts_with("1699")
}

fn theostone_icon(code: &str) -> Option<String> {
    if !code.starts_with("30") {
        return None;
    }
    let icon: u32 = code[5..7].parse().ok().filter(|&i| i > 0)?;
    Some(cdn(&format!("{THEOSTONE_ICON_BASE}{icon:03x}")))
}

fn class_code(prefix: &str) -> Option<&'static str> {
    Some(match prefix {
        "11" => "GL",
        "12" => "TE",
        "13" => "AS",
        "14" => "RA",
        "15" => "SO",
        "16" => "EL",
        "17" => "CL",
        "18" => "CH",
        "19" => "GT",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_wins_over_the_class_pattern() {
        let table = HashMap::from([("1101".to_string(), "ICON_GL_SKILL_042".to_string())]);
        assert_eq!(
            skill_icons(11_010_250, &table),
            vec![cdn("ICON_GL_SKILL_042")]
        );
        assert_eq!(
            skill_icons(17_050_000, &table),
            vec![cdn("ICON_CL_SKILL_005")]
        );
    }

    #[test]
    fn basic_attacks_and_unknown_codes_have_no_icon() {
        let table = HashMap::new();
        assert!(skill_icons(11_000_001, &table).is_empty());
        assert!(skill_icons(10_000, &table).is_empty());
        assert!(skill_icons(0, &table).is_empty());
        assert!(skill_icons(99_010_000, &table).is_empty());
    }

    #[test]
    fn theostones_use_the_item_icon() {
        assert_eq!(
            skill_icons(30_001_120, &HashMap::new()),
            vec![cdn("Icon_Item_Usable_Godstone_WP_r_00c")]
        );
    }

    #[test]
    fn buff_icons_fall_back_to_the_plain_name_then_the_skill() {
        let urls = abnormal_icons(
            Some("ICON_CL_SKILL_040_Buff"),
            Some(17_040_000),
            &HashMap::new(),
        );
        assert_eq!(
            urls,
            vec![
                cdn("ICON_CL_SKILL_040_Buff"),
                cdn("ICON_CL_SKILL_040"),
                cdn("ICON_CL_SKILL_004")
            ]
        );
    }
}
