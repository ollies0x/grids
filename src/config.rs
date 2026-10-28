//! The whole configuration as one JSON document. Every field has a default so an older or
//! hand-edited file still loads; the daemon writes the full document back on start so the
//! Settings app always sees every key.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const CMD: u64 = 1 << 20;
pub const OPT: u64 = 1 << 19;
pub const CTRL: u64 = 1 << 18;
pub const SHIFT: u64 = 1 << 17;
pub const FN: u64 = 1 << 23;
pub const MOD_MASK: u64 = CMD | OPT | CTRL | SHIFT | FN;

/// Same shape Rectangle stores in its defaults: a virtual key code and NSEvent modifier flags.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub key_code: u32,
    pub modifier_flags: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    pub version: u32,
    pub general: General,
    /// command -> shortcut. Commands are action ids ("left-half"), "custom:<name>",
    /// "layout:<name>", or app commands ("pin", "save-layout", "show-menu").
    pub shortcuts: BTreeMap<String, Shortcut>,
    /// Modifiers combined with the arrow keys for the Windows-style state machine. 0 = off.
    pub win_arrow_modifiers: u64,
    pub throw: ThrowCfg,
    pub quick_throw: Vec<QuickThrowRule>,
    pub move_resize: MoveResizeCfg,
    pub snap: SnapCfg,
    pub custom: Vec<CustomSpec>,
    pub layouts: Vec<Layout>,
    pub layout_save_as_presets: bool,
    pub stash: StashCfg,
    pub pin: PinCfg,
    pub menu: MenuCfg,
    pub tweaks: Tweaks,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct General {
    pub launch_at_login: bool,
    /// none | cycleDisplays | cycleSizes | adjacentDisplay | adjacentOrCycle
    pub repeat_mode: String,
    /// Fractions offered when cycling half actions, from: 2/3 1/2 1/3 1/4 3/4.
    pub cycle_sizes: Vec<String>,
    pub reset_repeat_on_release: bool,
    pub allow_any_shortcut: bool,
    pub gap: f64,
    pub gaps_on_edges: bool,
    pub move_cursor_across_displays: bool,
    pub move_cursor_always: bool,
    pub double_click_title_bar: bool,
    pub resize_adjacent: bool,
    pub adjust_for_dock: bool,
    pub restore_display_layout: bool,
    /// Width kept clear for Stage Manager's strip; 0 ignores Stage Manager. <=1 is a fraction.
    pub stage_manager_area: f64,
    pub icloud_sync: bool,
    pub ignored_apps: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct ThrowCfg {
    /// Held modifiers that start the Window Throw. 0 = off.
    pub modifiers: u64,
    /// Mouse button number that starts it (2 = middle). None = off.
    pub mouse_button: Option<i64>,
    /// Touches to rest before lifting all but one (3, 4 or 5). None = off.
    pub trackpad_fingers: Option<u8>,
    /// Commands for the eight pie sectors, clockwise from up: N NE E SE S SW W NW.
    pub sectors: Vec<String>,
    pub long_sectors: Vec<String>,
    pub long_throw: bool,
    pub scroll_resizes: bool,
    pub ignore_without_toolbar: bool,
    pub reticle_size: f64,
    pub reticle_color: [f64; 3],
    pub reticle_ring: bool,
    pub safe_area: f64,
    pub min_dist_to_appear: f64,
    pub long_throw_distance: f64,
    pub other_display_center: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct QuickThrowRule {
    pub modifiers: u64,
    pub up: Option<String>,
    pub down: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct MoveResizeCfg {
    pub move_modifiers: u64,
    pub resize_modifiers: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SnapCfg {
    pub enabled: bool,
    pub restore_on_unsnap: bool,
    pub animate: bool,
    pub haptic: bool,
    /// Modifiers required for drag-to-snap. 0 = none required.
    pub modifiers: u64,
    /// Area -> command. Areas: top bottom left right topLeft topRight bottomLeft bottomRight.
    /// Values are commands or compound edge modes ("compound:thirds", "compound:halves",
    /// "compound:sixths", "compound:fourths", "compound:half-corners"), or "" for none.
    pub edges: BTreeMap<String, String>,
    pub portrait_edges: BTreeMap<String, String>,
    /// always | whilePressing | whileNotPressing
    pub targets_mode: String,
    pub targets_modifiers: u64,
    pub panel_enabled: bool,
    /// above below left right top bottom topLeft topRight bottomLeft bottomRight
    pub panel_position: String,
    pub panel_items: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Placement {
    /// center top bottom left right topLeft topRight bottomLeft bottomRight custom centerOffset
    pub position: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub w: Option<f64>,
    pub h: Option<f64>,
}

/// One execution of a custom entry or layout entry: a preset command or a placement, and
/// where. `display` is unchanged | next | previous | name:<name> | number:<n>.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Step {
    pub command: Option<String>,
    pub place: Option<Placement>,
    pub display: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CustomSpec {
    pub name: String,
    /// First step runs on the first execution, later steps on repeats.
    pub steps: Vec<Step>,
    pub snap_target: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct LayoutEntry {
    /// Bundle id, or "*" for any app.
    pub app: String,
    pub app_name: String,
    pub steps: Vec<Step>,
    pub snap_target: bool,
    /// any | loose | contains | regex
    pub title_mode: String,
    pub title: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Layout {
    pub name: String,
    /// Display name, "*" for any, None = not a trigger.
    pub on_display_connected: Option<String>,
    pub on_display_disconnected: Option<String>,
    pub on_wake: bool,
    pub on_window_opens: bool,
    pub frontmost_only: bool,
    pub all_windows: bool,
    pub bring_to_front: bool,
    pub launch_apps: bool,
    /// none | hide | minimize | quit
    pub others: String,
    pub entries: Vec<LayoutEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct StashCfg {
    pub show_delay: f64,
    pub show_requires_cmd: bool,
    pub hide_delay: f64,
    pub hide_on_leave: bool,
    pub hide_requires_cmd: bool,
    pub unhide_on_activate: bool,
    pub unstash_on_move: bool,
    pub unstash_on_action: bool,
    pub animate: bool,
    pub hide_in_corner: bool,
    pub color_tabs: bool,
    /// Pixels of a stashed window left on screen.
    pub sliver: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct PinCfg {
    pub enabled: bool,
    pub show_in_menu: bool,
    pub app: String,
    pub app_name: String,
    /// <=1 fraction of the screen width, otherwise pixels.
    pub width: f64,
    /// left | right
    pub side: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct MenuCfg {
    pub hide_icon: bool,
    pub icon_variant: u8,
    /// Left click with these modifiers shows the menu for the window under the cursor. 0 = off.
    pub click_modifiers: u64,
    pub hidden_categories: Vec<String>,
    pub submenu_categories: Vec<String>,
    pub hidden_actions: Vec<String>,
    pub show_custom: bool,
    pub show_layouts: bool,
    pub show_icons: bool,
}

/// Rectangle's Terminal-only preferences, kept as plain settings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Tweaks {
    pub almost_maximize_width: f64,
    pub almost_maximize_height: f64,
    pub size_offset: f64,
    pub nudge_offset: f64,
    pub minimum_window_width: f64,
    pub minimum_window_height: f64,
    pub curtain_resize: bool,
    pub snap_edge_margin: [f64; 4],
    pub corner_size: f64,
    pub short_edge: f64,
    /// top bottom left right
    pub screen_edge_gaps: [f64; 4],
    pub screen_edge_gap_top_notch: Option<f64>,
    pub screen_edge_gaps_main_only: bool,
    pub apply_gaps_to_maximize: bool,
    pub apply_gaps_to_maximize_height: bool,
    pub double_click_restore: bool,
    pub double_click_ignored_apps: Vec<String>,
    pub screens_ordered_by_x: bool,
    pub footprint_alpha: f64,
    pub footprint_border_width: f64,
    pub footprint_color: [f64; 3],
    pub footprint_fade: bool,
    pub footprint_animation: f64,
    pub centered_directional_move: bool,
    pub resize_on_directional_move: bool,
    pub center_half_cycles: bool,
    pub specified_width: f64,
    pub specified_height: f64,
    pub reveal_desktop_width: f64,
    pub cascade_offset: f64,
    pub space_switch_left: Shortcut,
    pub space_switch_right: Shortcut,
}

fn s(v: &str) -> String {
    v.to_string()
}

fn strmap(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs.iter().map(|(k, v)| (s(k), s(v))).collect()
}

impl Default for General {
    fn default() -> Self {
        General {
            launch_at_login: false,
            repeat_mode: s("cycleSizes"),
            cycle_sizes: vec![s("2/3"), s("1/2"), s("1/3")],
            reset_repeat_on_release: false,
            allow_any_shortcut: false,
            gap: 0.0,
            gaps_on_edges: true,
            move_cursor_across_displays: false,
            move_cursor_always: false,
            double_click_title_bar: false,
            resize_adjacent: false,
            adjust_for_dock: false,
            restore_display_layout: true,
            stage_manager_area: 190.0,
            icloud_sync: false,
            ignored_apps: vec![],
        }
    }
}

impl Default for ThrowCfg {
    fn default() -> Self {
        ThrowCfg {
            modifiers: CTRL | CMD,
            mouse_button: None,
            trackpad_fingers: None,
            sectors: [
                "maximize", "top-right", "right-half", "bottom-right", "center", "bottom-left",
                "left-half", "top-left",
            ]
            .map(s)
            .to_vec(),
            long_sectors: [
                "top-half", "top-right-sixth", "last-third", "bottom-right-sixth", "bottom-half",
                "bottom-left-sixth", "first-third", "top-left-sixth",
            ]
            .map(s)
            .to_vec(),
            long_throw: true,
            scroll_resizes: true,
            ignore_without_toolbar: false,
            reticle_size: 26.0,
            reticle_color: [0.45, 0.62, 1.0],
            reticle_ring: true,
            safe_area: 18.0,
            min_dist_to_appear: 0.0,
            long_throw_distance: 220.0,
            other_display_center: true,
        }
    }
}

impl Default for SnapCfg {
    fn default() -> Self {
        let edges = strmap(&[
            ("top", "maximize"),
            ("bottom", "compound:thirds"),
            ("left", "compound:half-corners"),
            ("right", "compound:half-corners"),
            ("topLeft", "top-left"),
            ("topRight", "top-right"),
            ("bottomLeft", "bottom-left"),
            ("bottomRight", "bottom-right"),
        ]);
        let portrait_edges = strmap(&[
            ("top", "maximize"),
            ("bottom", "compound:thirds"),
            ("left", "top-half"),
            ("right", "bottom-half"),
            ("topLeft", "top-left"),
            ("topRight", "top-right"),
            ("bottomLeft", "bottom-left"),
            ("bottomRight", "bottom-right"),
        ]);
        SnapCfg {
            enabled: true,
            restore_on_unsnap: true,
            animate: true,
            haptic: true,
            modifiers: 0,
            edges,
            portrait_edges,
            targets_mode: s("always"),
            targets_modifiers: 0,
            panel_enabled: false,
            panel_position: s("above"),
            panel_items: ["left-half", "maximize", "right-half", "first-third", "center-third", "last-third"]
                .map(s)
                .to_vec(),
        }
    }
}

impl Default for StashCfg {
    fn default() -> Self {
        StashCfg {
            show_delay: 0.0,
            show_requires_cmd: false,
            hide_delay: 0.2,
            hide_on_leave: true,
            hide_requires_cmd: false,
            unhide_on_activate: true,
            unstash_on_move: true,
            unstash_on_action: true,
            animate: true,
            hide_in_corner: false,
            color_tabs: true,
            sliver: 6.0,
        }
    }
}

impl Default for PinCfg {
    fn default() -> Self {
        PinCfg { enabled: false, show_in_menu: true, app: s(""), app_name: s(""), width: 400.0, side: s("right") }
    }
}

impl Default for MenuCfg {
    fn default() -> Self {
        MenuCfg {
            hide_icon: false,
            icon_variant: 0,
            click_modifiers: 0,
            hidden_categories: [
                "Fifths", "Eighths", "Ninths", "Corner Two Thirds", "Additional Thirds", "Snap", "Nudge",
                "Double & Halve", "Control",
            ]
            .map(s)
            .to_vec(),
            submenu_categories: [
                "Fill", "Fourths", "Sixths", "Size", "Multiple Windows", "Stash", "Move",
            ]
            .map(s)
            .to_vec(),
            hidden_actions: vec![],
            show_custom: true,
            show_layouts: true,
            show_icons: true,
        }
    }
}

impl Default for Tweaks {
    fn default() -> Self {
        Tweaks {
            almost_maximize_width: 0.9,
            almost_maximize_height: 0.9,
            size_offset: 30.0,
            nudge_offset: 30.0,
            minimum_window_width: 0.25,
            minimum_window_height: 0.25,
            curtain_resize: true,
            snap_edge_margin: [5.0, 5.0, 5.0, 5.0],
            corner_size: 20.0,
            short_edge: 145.0,
            screen_edge_gaps: [0.0; 4],
            screen_edge_gap_top_notch: None,
            screen_edge_gaps_main_only: false,
            apply_gaps_to_maximize: true,
            apply_gaps_to_maximize_height: true,
            double_click_restore: true,
            double_click_ignored_apps: vec![],
            screens_ordered_by_x: false,
            footprint_alpha: 0.3,
            footprint_border_width: 2.0,
            footprint_color: [0.45, 0.62, 1.0],
            footprint_fade: true,
            footprint_animation: 0.0,
            centered_directional_move: true,
            resize_on_directional_move: false,
            center_half_cycles: false,
            specified_width: 1680.0,
            specified_height: 1050.0,
            reveal_desktop_width: 160.0,
            cascade_offset: 28.0,
            // Control-Left / Control-Right, the macOS default Space switch shortcuts.
            space_switch_left: Shortcut { key_code: 123, modifier_flags: CTRL },
            space_switch_right: Shortcut { key_code: 124, modifier_flags: CTRL },
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            version: 1,
            general: General::default(),
            shortcuts: default_shortcuts(),
            win_arrow_modifiers: 0,
            throw: ThrowCfg::default(),
            quick_throw: vec![],
            move_resize: MoveResizeCfg::default(),
            snap: SnapCfg::default(),
            custom: vec![],
            layouts: vec![],
            layout_save_as_presets: true,
            stash: StashCfg::default(),
            pin: PinCfg::default(),
            menu: MenuCfg::default(),
            tweaks: Tweaks::default(),
        }
    }
}

/// Rectangle's recommended (non-Spectacle) defaults.
pub fn default_shortcuts() -> BTreeMap<String, Shortcut> {
    use crate::keys::*;
    let co = CTRL | OPT;
    let coc = CTRL | OPT | CMD;
    let pairs: &[(&str, u32, u64)] = &[
        ("left-half", K_LEFT, co),
        ("right-half", K_RIGHT, co),
        ("top-half", K_UP, co),
        ("bottom-half", K_DOWN, co),
        ("top-left", K_U, co),
        ("top-right", K_I, co),
        ("bottom-left", K_J, co),
        ("bottom-right", K_K, co),
        ("first-third", K_D, co),
        ("center-third", K_F, co),
        ("last-third", K_G, co),
        ("first-two-thirds", K_E, co),
        ("center-two-thirds", K_R, co),
        ("last-two-thirds", K_T, co),
        ("maximize", K_RETURN, co),
        ("maximize-height", K_UP, co | SHIFT),
        ("smaller", K_MINUS, co),
        ("larger", K_EQUAL, co),
        ("center", K_C, co),
        ("restore", K_DELETE, co),
        ("next-display", K_RIGHT, coc),
        ("previous-display", K_LEFT, coc),
    ];
    pairs
        .iter()
        .map(|(id, k, m)| (s(id), Shortcut { key_code: *k, modifier_flags: *m }))
        .collect()
}

pub fn support_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join("Library/Application Support/Grids")
}

pub fn icloud_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join("Library/Mobile Documents/com~apple~CloudDocs/Grids")
}

pub fn local_path() -> PathBuf {
    support_dir().join("config.json")
}

/// Grids was called Mullion; take its config and support files with it on first run.
fn migrate_from_mullion() {
    for (old, new) in [("Library/Application Support/Mullion", support_dir()), ("Library/Mobile Documents/com~apple~CloudDocs/Mullion", icloud_dir())] {
        let old = std::env::var("HOME").map(|h| PathBuf::from(h).join(old)).unwrap_or_default();
        if !new.exists() && old.exists() {
            let _ = std::fs::rename(&old, &new);
        }
    }
}

/// The file actually in use: the iCloud copy when sync is on (marked by a flag file so the
/// choice itself does not need to sync), otherwise the local one.
pub fn active_path() -> PathBuf {
    if support_dir().join("icloud-sync").exists() {
        icloud_dir().join("config.json")
    } else {
        local_path()
    }
}

impl Config {
    pub fn load() -> Config {
        migrate_from_mullion();
        let p = active_path();
        match std::fs::read(&p) {
            Ok(bytes) => match serde_json::from_slice::<Config>(&bytes) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("grids: config {} unreadable ({e}); keeping a copy and using defaults", p.display());
                    let _ = std::fs::copy(&p, p.with_extension("json.bad"));
                    Config::default()
                }
            },
            Err(_) => {
                let mut c = Config::default();
                crate::import::from_rectangle(&mut c);
                c
            }
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let p = active_path();
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self).unwrap())?;
        std::fs::rename(&tmp, &p)
    }

    pub fn custom(&self, name: &str) -> Option<&CustomSpec> {
        self.custom.iter().find(|c| c.name == name)
    }
    pub fn layout(&self, name: &str) -> Option<&Layout> {
        self.layouts.iter().find(|l| l.name == name)
    }
}

/// Tell an open Settings window that the daemon changed the file.
pub fn announce_change() {
    use crate::ffi::*;
    let n = cfstr("app.grids.changed");
    unsafe { CFNotificationCenterPostNotification(CFNotificationCenterGetDistributedCenter(), n.ptr(), std::ptr::null(), std::ptr::null(), 1) };
}

/// "2/3" -> 0.666..; also accepts decimals.
pub fn parse_frac(v: &str) -> Option<f64> {
    if let Some((a, b)) = v.split_once('/') {
        let a: f64 = a.trim().parse().ok()?;
        let b: f64 = b.trim().parse().ok()?;
        if b == 0.0 { None } else { Some(a / b) }
    } else {
        v.trim().parse().ok()
    }
}
