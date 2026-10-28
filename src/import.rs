//! First-run import of an existing Rectangle (or Rectangle Pro) setup, so switching keeps the
//! user's shortcuts, gaps and repeat behaviour.

use crate::actions::ACTIONS;
use crate::config::{Config, Shortcut};
use crate::ffi::*;

const DOMAINS: [&str; 2] = ["com.knollsoft.Hookshot", "com.knollsoft.Rectangle"];

fn camel(id: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for ch in id.chars() {
        if ch == '-' {
            up = true;
        } else if up {
            out.extend(ch.to_uppercase());
            up = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn shortcut(domain: &str, key: &str) -> Option<Shortcut> {
    let v = pref_value(domain, key)?;
    let code = cf_to_f64(dict_get(v.ptr(), "keyCode"))?;
    let mods = cf_to_f64(dict_get(v.ptr(), "modifierFlags"))?;
    Some(Shortcut { key_code: code as u32, modifier_flags: mods as u64 & crate::config::MOD_MASK })
}

fn num(domain: &str, key: &str) -> Option<f64> {
    pref_value(domain, key).and_then(|v| cf_to_f64(v.ptr()))
}

pub fn from_rectangle(c: &mut Config) -> bool {
    let Some(domain) = DOMAINS.iter().copied().find(|d| pref_value(d, "installVersion").is_some() || pref_value(d, "lastVersion").is_some()) else {
        return false;
    };
    for a in ACTIONS {
        if let Some(sc) = shortcut(domain, &camel(a.id)) {
            if sc.key_code != 0 || sc.modifier_flags != 0 {
                c.shortcuts.insert(a.id.to_string(), sc);
            }
        }
    }
    // Rectangle's own names that differ from the URL ids, including the Pro-only extra
    // sizes and the Landscape/Portrait variants of the sixth/fifth bands.
    const PRO_EXTRA: &[&str] = &[
        "previousDisplay", "centerProminently", "leftThird", "rightThird", "centerVerticalThird",
        "middleVerticalThird", "centerVerticalTwoThirds", "leftTwoThirds", "rightTwoThirds",
        "centerHorizontalThird", "centerHorizontalTwoThirds", "leftFourth", "centerLeftFourth",
        "centerRightFourth", "rightFourth", "leftThreeFourths", "rightThreeFourths",
        "topVerticalThird", "bottomVerticalThird", "topVerticalTwoThirds", "bottomVerticalTwoThirds",
        "centerVerticalThreeFourths", "centerHorizontalThreeFourths", "centerVerticalHalf",
        "centerHorizontalHalf", "tileActiveApp", "cascadeActiveApp",
        "topLeftSixthLandscape", "topCenterSixthLandscape", "topRightSixthLandscape",
        "bottomLeftSixthLandscape", "bottomCenterSixthLandscape", "bottomRightSixthLandscape",
        "topLeftSixthPortrait", "topRightSixthPortrait", "leftCenterSixthPortrait",
        "rightCenterSixthPortrait", "bottomLeftSixthPortrait", "bottomRightSixthPortrait",
        "topLeftTwoSixthsLandscape", "topLeftTwoSixthsPortrait", "topRightTwoSixthsLandscape",
        "topRightTwoSixthsPortrait", "bottomLeftTwoSixthsLandscape", "bottomLeftTwoSixthsPortrait",
        "bottomRightTwoSixthsLandscape", "bottomRightTwoSixthsPortrait",
        "firstFifthLandscape", "secondFifthLandscape", "thirdFifthLandscape", "fourthFifthLandscape",
        "lastFifthLandscape", "firstFifthPortrait", "secondFifthPortrait", "thirdFifthPortrait",
        "fourthFifthPortrait", "lastFifthPortrait",
        "firstSixthLandscape", "secondSixthLandscape", "thirdSixthLandscape", "fourthSixthLandscape",
        "fifthSixthLandscape", "lastSixthLandscape", "firstSixthPortrait", "secondSixthPortrait",
        "thirdSixthPortrait", "fourthSixthPortrait", "fifthSixthPortrait", "lastSixthPortrait",
        "unstashLeft", "unstashRight", "unstashUp", "unstashDown",
        "cycleStashLeft", "cycleStashRight", "cycleStashUp", "cycleStashDown",
        "toggleStashLeft", "toggleStashRight", "toggleStashUp", "toggleStashDown",
    ];
    for raw in PRO_EXTRA {
        let id = crate::actions::resolve(raw);
        if id.is_empty() {
            continue;
        }
        if let Some(sc) = shortcut(domain, raw) {
            if sc.key_code != 0 || sc.modifier_flags != 0 {
                c.shortcuts.entry(id).or_insert(sc);
            }
        }
    }
    if let Some(m) = num(domain, "subsequentExecutionMode") {
        c.general.repeat_mode = match m as i64 {
            0 => "cycleSizes",
            1 => "adjacentDisplay",
            2 => "none",
            3 => "adjacentOrCycle",
            4 => "cycleDisplays",
            _ => "cycleSizes",
        }
        .into();
    }
    if let Some(g) = num(domain, "gapSize") {
        c.general.gap = g;
    }
    if let Some(v) = num(domain, "windowSnapping") {
        c.snap.enabled = v as i64 != 2;
    }
    if let Some(v) = num(domain, "almostMaximizeWidth") {
        c.tweaks.almost_maximize_width = v;
    }
    if let Some(v) = num(domain, "almostMaximizeHeight") {
        c.tweaks.almost_maximize_height = v;
    }
    if let Some(v) = num(domain, "sizeOffset") {
        c.tweaks.size_offset = v;
    }
    if let Some(v) = num(domain, "centerHalfCycles") {
        c.tweaks.center_half_cycles = v as i64 == 1;
    }
    if let Some(v) = num(domain, "stageSize") {
        c.general.stage_manager_area = v;
    }
    if let Some(v) = pref_value(domain, "allowAnyShortcut").and_then(|v| cf_to_bool(v.ptr())) {
        c.general.allow_any_shortcut = v;
    }
    if let Some(v) = num(domain, "snapModifiers") {
        c.snap.modifiers = v as u64;
    }
    eprintln!("grids: imported settings from {domain}");
    true
}
