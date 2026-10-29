//! The action catalog and the pure geometry for every single-window action. Multi-window,
//! display, stash, pin and control actions are carried out by the engine, but they are
//! listed here so menus, the URL API and Settings share one catalog.

use crate::config::Tweaks;
use crate::geom::Rect;

pub struct Action {
    pub id: &'static str,
    pub name: &'static str,
    pub cat: &'static str,
}

const fn a(id: &'static str, name: &'static str, cat: &'static str) -> Action {
    Action { id, name, cat }
}

pub const CATEGORIES: &[&str] = &[
    "Fill", "Maximize", "Halves", "Corners", "Thirds", "Additional Thirds", "Corner Two Thirds",
    "Corner Thirds", "Fourths", "Fifths", "Sixths", "Eighths", "Ninths", "Display", "Move", "Snap",
    "Nudge", "Size",
    "Double & Halve", "Other", "Control", "Multiple Windows", "Stash", "Pin", "App",
];

pub const ACTIONS: &[Action] = &[
    a("fill-left", "Fill Left", "Fill"),
    a("fill-right", "Fill Right", "Fill"),
    a("fill-top-left", "Fill Top Left", "Fill"),
    a("fill-top-right", "Fill Top Right", "Fill"),
    a("fill-bottom-left", "Fill Bottom Left", "Fill"),
    a("fill-bottom-right", "Fill Bottom Right", "Fill"),
    a("maximize", "Maximize", "Maximize"),
    a("almost-maximize", "Almost Maximize", "Maximize"),
    a("maximize-height", "Maximize Height", "Maximize"),
    a("maximize-width", "Maximize Width", "Maximize"),
    a("left-half", "Left Half", "Halves"),
    a("right-half", "Right Half", "Halves"),
    a("center-half", "Center Half", "Halves"),
    a("top-half", "Top Half", "Halves"),
    a("bottom-half", "Bottom Half", "Halves"),
    a("top-left", "Top Left", "Corners"),
    a("top-right", "Top Right", "Corners"),
    a("bottom-left", "Bottom Left", "Corners"),
    a("bottom-right", "Bottom Right", "Corners"),
    a("first-third", "First Third", "Thirds"),
    a("center-third", "Center Third", "Thirds"),
    a("last-third", "Last Third", "Thirds"),
    a("first-two-thirds", "First Two Thirds", "Thirds"),
    a("center-two-thirds", "Center Two Thirds", "Thirds"),
    a("last-two-thirds", "Last Two Thirds", "Thirds"),
    a("top-third", "Top Third", "Additional Thirds"),
    a("vertical-center-third", "Center Third (Vertical)", "Additional Thirds"),
    a("bottom-third", "Bottom Third", "Additional Thirds"),
    a("top-two-thirds", "Top Two Thirds", "Additional Thirds"),
    a("bottom-two-thirds", "Bottom Two Thirds", "Additional Thirds"),
    a("center-horizontal-third", "Center Third (Horizontal)", "Additional Thirds"),
    a("center-vertical-two-thirds", "Center Two Thirds (Vertical)", "Additional Thirds"),
    a("center-horizontal-two-thirds", "Center Two Thirds (Horizontal)", "Additional Thirds"),
    a("top-left-third", "Top Left Two Thirds", "Corner Two Thirds"),
    a("top-right-third", "Top Right Two Thirds", "Corner Two Thirds"),
    a("bottom-left-third", "Bottom Left Two Thirds", "Corner Two Thirds"),
    a("bottom-right-third", "Bottom Right Two Thirds", "Corner Two Thirds"),
    a("first-fourth", "First Fourth", "Fourths"),
    a("second-fourth", "Second Fourth", "Fourths"),
    a("third-fourth", "Third Fourth", "Fourths"),
    a("last-fourth", "Last Fourth", "Fourths"),
    a("first-three-fourths", "First Three Fourths", "Fourths"),
    a("center-three-fourths", "Center Three Fourths", "Fourths"),
    a("last-three-fourths", "Last Three Fourths", "Fourths"),
    a("top-fourth", "Top Fourth", "Fourths"),
    a("center-top-fourth", "Center Top Fourth", "Fourths"),
    a("center-bottom-fourth", "Center Bottom Fourth", "Fourths"),
    a("bottom-fourth", "Bottom Fourth", "Fourths"),
    a("top-three-fourths", "Top Three Fourths", "Fourths"),
    a("bottom-three-fourths", "Bottom Three Fourths", "Fourths"),
    a("center-vertical-three-fourths", "Center Three Fourths (Vertical)", "Fourths"),
    a("center-horizontal-three-fourths", "Center Three Fourths (Horizontal)", "Fourths"),
    a("center-vertical-half", "Center Half (Vertical)", "Fourths"),
    a("top-left-two-sixths", "Top Left Corner Third", "Corner Thirds"),
    a("top-right-two-sixths", "Top Right Corner Third", "Corner Thirds"),
    a("bottom-left-two-sixths", "Bottom Left Corner Third", "Corner Thirds"),
    a("bottom-right-two-sixths", "Bottom Right Corner Third", "Corner Thirds"),
    a("first-fifth", "First Fifth", "Fifths"),
    a("second-fifth", "Second Fifth", "Fifths"),
    a("third-fifth", "Third Fifth", "Fifths"),
    a("fourth-fifth", "Fourth Fifth", "Fifths"),
    a("last-fifth", "Last Fifth", "Fifths"),
    a("top-left-sixth", "Top Left Sixth", "Sixths"),
    a("top-center-sixth", "Top Center Sixth", "Sixths"),
    a("top-right-sixth", "Top Right Sixth", "Sixths"),
    a("bottom-left-sixth", "Bottom Left Sixth", "Sixths"),
    a("bottom-center-sixth", "Bottom Center Sixth", "Sixths"),
    a("bottom-right-sixth", "Bottom Right Sixth", "Sixths"),
    a("first-sixth", "First Sixth", "Sixths"),
    a("second-sixth", "Second Sixth", "Sixths"),
    a("third-sixth", "Third Sixth", "Sixths"),
    a("fourth-sixth", "Fourth Sixth", "Sixths"),
    a("fifth-sixth", "Fifth Sixth", "Sixths"),
    a("last-sixth", "Last Sixth", "Sixths"),
    a("left-center-sixth", "Left Center Sixth", "Sixths"),
    a("right-center-sixth", "Right Center Sixth", "Sixths"),
    a("top-left-eighth", "Top Left Eighth", "Eighths"),
    a("top-center-left-eighth", "Top Center Left Eighth", "Eighths"),
    a("top-center-right-eighth", "Top Center Right Eighth", "Eighths"),
    a("top-right-eighth", "Top Right Eighth", "Eighths"),
    a("bottom-left-eighth", "Bottom Left Eighth", "Eighths"),
    a("bottom-center-left-eighth", "Bottom Center Left Eighth", "Eighths"),
    a("bottom-center-right-eighth", "Bottom Center Right Eighth", "Eighths"),
    a("bottom-right-eighth", "Bottom Right Eighth", "Eighths"),
    a("top-left-ninth", "Top Left Ninth", "Ninths"),
    a("top-center-ninth", "Top Center Ninth", "Ninths"),
    a("top-right-ninth", "Top Right Ninth", "Ninths"),
    a("middle-left-ninth", "Middle Left Ninth", "Ninths"),
    a("middle-center-ninth", "Middle Center Ninth", "Ninths"),
    a("middle-right-ninth", "Middle Right Ninth", "Ninths"),
    a("bottom-left-ninth", "Bottom Left Ninth", "Ninths"),
    a("bottom-center-ninth", "Bottom Center Ninth", "Ninths"),
    a("bottom-right-ninth", "Bottom Right Ninth", "Ninths"),
    a("next-display", "Next Display", "Display"),
    a("previous-display", "Previous Display", "Display"),
    a("next-display-ratio", "Next Display Ratio", "Display"),
    a("prev-display-ratio", "Previous Display Ratio", "Display"),
    a("prev-display-max", "Previous Display Maximized", "Display"),
    a("next-display-max", "Next Display Maximized", "Display"),
    a("next-space", "Next Space", "Display"),
    a("prev-space", "Previous Space", "Display"),
    a("move-left", "Move Left", "Move"),
    a("move-right", "Move Right", "Move"),
    a("move-up", "Move Up", "Move"),
    a("move-down", "Move Down", "Move"),
    a("snap-left", "Snap Left", "Snap"),
    a("snap-right", "Snap Right", "Snap"),
    a("snap-up", "Snap Up", "Snap"),
    a("snap-down", "Snap Down", "Snap"),
    a("snap-top-left", "Snap Top Left", "Snap"),
    a("snap-top-right", "Snap Top Right", "Snap"),
    a("snap-bottom-left", "Snap Bottom Left", "Snap"),
    a("snap-bottom-right", "Snap Bottom Right", "Snap"),
    a("nudge-left", "Nudge Left", "Nudge"),
    a("nudge-right", "Nudge Right", "Nudge"),
    a("nudge-up", "Nudge Up", "Nudge"),
    a("nudge-down", "Nudge Down", "Nudge"),
    a("larger", "Larger", "Size"),
    a("smaller", "Smaller", "Size"),
    a("larger-width", "Larger Width", "Size"),
    a("smaller-width", "Smaller Width", "Size"),
    a("larger-height", "Larger Height", "Size"),
    a("smaller-height", "Smaller Height", "Size"),
    a("double-width-left", "Double Width Left", "Double & Halve"),
    a("double-width-right", "Double Width Right", "Double & Halve"),
    a("double-height-up", "Double Height Up", "Double & Halve"),
    a("double-height-down", "Double Height Down", "Double & Halve"),
    a("halve-width-left", "Halve Width Left", "Double & Halve"),
    a("halve-width-right", "Halve Width Right", "Double & Halve"),
    a("halve-height-up", "Halve Height Up", "Double & Halve"),
    a("halve-height-down", "Halve Height Down", "Double & Halve"),
    a("center", "Center", "Other"),
    a("upper-center", "Upper Center", "Other"),
    a("specified", "Specified Size", "Other"),
    a("restore", "Restore", "Other"),
    a("last", "Last", "Other"),
    a("restore-to-screen", "Restore to Previous Screen", "Control"),
    a("reasonable-size", "Reasonable Size", "Other"),
    a("focus-window-under-cursor", "Focus Window Under Cursor", "Control"),
    a("center-on-cursor-display", "Center on Cursor Display", "Control"),
    a("open-settings", "Open Settings", "App"),
    a("fullscreen", "Full Screen", "Control"),
    a("close", "Close", "Control"),
    a("minimize", "Minimize", "Control"),
    a("quit-app", "Quit App", "Control"),
    a("hide-app", "Hide App", "Control"),
    a("reveal-desktop-edge", "Reveal Desktop Edge", "Multiple Windows"),
    a("hide-all", "Hide All Apps", "Multiple Windows"),
    a("unminimize-all-windows", "Unminimize All Windows", "Multiple Windows"),
    a("tile2x2", "2x2 Tiles", "Multiple Windows"),
    a("tile2x3", "2x3 Tiles", "Multiple Windows"),
    a("tile1x2", "1x2 Tiles", "Multiple Windows"),
    a("tile2x1", "2x1 Tiles", "Multiple Windows"),
    a("tile1x3", "1x3 Tiles", "Multiple Windows"),
    a("tile1x4", "1x4 Tiles", "Multiple Windows"),
    a("reverse-all", "Reverse All Windows", "Multiple Windows"),
    a("app-collate-left", "Collate App Left", "Multiple Windows"),
    a("app-collate-right", "Collate App Right", "Multiple Windows"),
    a("left-and-quarters", "Left and Quarters", "Multiple Windows"),
    a("right-and-quarters", "Right and Quarters", "Multiple Windows"),
    a("tile-all", "Tile All Windows", "Multiple Windows"),
    a("undo-grid", "Undo Grid", "Multiple Windows"),
    a("tile-app", "Tile App Windows", "Multiple Windows"),
    a("cascade-all", "Cascade All Windows", "Multiple Windows"),
    a("cascade-app", "Cascade App Windows", "Multiple Windows"),
    a("app-next-display", "App Next Display", "Multiple Windows"),
    a("app-prev-display", "App Previous Display", "Multiple Windows"),
    a("app-left-half", "App Left Half", "Multiple Windows"),
    a("app-right-half", "App Right Half", "Multiple Windows"),
    a("tidy", "Tidy", "Multiple Windows"),
    a("stash-left", "Stash Left", "Stash"),
    a("stash-right", "Stash Right", "Stash"),
    a("stash-up", "Stash Up", "Stash"),
    a("stash-down", "Stash Down", "Stash"),
    a("unstash", "Unstash", "Stash"),
    a("stash-all", "Stash All", "Stash"),
    a("stash-all-but-front", "Stash All Except One", "Stash"),
    a("cycle-stashed", "Cycle Stashed", "Stash"),
    a("toggle-stashed", "Toggle Stashed", "Stash"),
    a("unstash-all", "Unstash All", "Stash"),
    a("unstash-left", "Unstash Left", "Stash"),
    a("unstash-right", "Unstash Right", "Stash"),
    a("unstash-up", "Unstash Up", "Stash"),
    a("unstash-down", "Unstash Down", "Stash"),
    a("cycle-stash-left", "Cycle Stash Left", "Stash"),
    a("cycle-stash-right", "Cycle Stash Right", "Stash"),
    a("cycle-stash-up", "Cycle Stash Up", "Stash"),
    a("cycle-stash-down", "Cycle Stash Down", "Stash"),
    a("toggle-stash-left", "Toggle Stash Left", "Stash"),
    a("toggle-stash-right", "Toggle Stash Right", "Stash"),
    a("toggle-stash-up", "Toggle Stash Up", "Stash"),
    a("toggle-stash-down", "Toggle Stash Down", "Stash"),
    a("pin", "Toggle Pin Mode", "Pin"),
    a("pin-left", "Pin Front App Left", "Pin"),
    a("pin-right", "Pin Front App Right", "Pin"),
    a("reflow-pin", "Reflow Pin", "Pin"),
    a("save-layout", "Save Current Layout", "App"),
    a("show-menu", "Show Menu at Cursor", "App"),
];

pub fn find(id: &str) -> Option<&'static Action> {
    let id = resolve(id);
    ACTIONS.iter().find(|a| a.id == id)
}

/// Names accepted from Rectangle / Rectangle Pro URLs that differ from ours.
pub fn alias(id: &str) -> &str {
    match id {
        "prev-display" => "previous-display",
        "center-prominently" => "upper-center",
        "stash-all-except-one" => "stash-all-but-front",
        "cycle-all-stashed" => "cycle-stashed",
        "toggle-all-stashed" => "toggle-stashed",
        "previous-space" => "prev-space",
        "previous-display-ratio" => "prev-display-ratio",
        // Rectangle Pro ids whose names differ or that we fold onto one action.
        "left-third" => "first-third",
        "right-third" => "last-third",
        "left-two-thirds" => "first-two-thirds",
        "right-two-thirds" => "last-two-thirds",
        "center-vertical-third" | "middle-vertical-third" => "vertical-center-third",
        "top-vertical-third" => "top-third",
        "bottom-vertical-third" => "bottom-third",
        "top-vertical-two-thirds" => "top-two-thirds",
        "bottom-vertical-two-thirds" => "bottom-two-thirds",
        "left-fourth" => "first-fourth",
        "center-left-fourth" => "second-fourth",
        "center-right-fourth" => "third-fourth",
        "right-fourth" => "last-fourth",
        "left-three-fourths" => "first-three-fourths",
        "right-three-fourths" => "last-three-fourths",
        "center-horizontal-half" => "center-half",
        "unreveal-desktop-edge" => "reveal-desktop-edge",
        "maximize-no-stage" => "maximize",
        "tile-active-app" => "tile-app",
        "cascade-active-app" => "cascade-app",
        "win-up" => "move-up",
        "win-down" => "move-down",
        "win-left" => "move-left",
        "win-right" => "move-right",
        "none" => "",
        "alternate-left-half" => "left-half",
        "reasonable-size" => "reasonable-size",
        "left-todo" => "pin-left",
        "right-todo" => "pin-right",
        other => other,
    }
}

/// Canonical catalog id for any accepted spelling: our kebab-case id, an alias, or
/// Rectangle/Rectangle Pro's camelCase raw name (with its orientation suffix folded in).
pub fn resolve(id: &str) -> String {
    let id = alias(id);
    if ACTIONS.iter().any(|a| a.id == id) {
        return id.to_string();
    }
    let mut kebab = String::with_capacity(id.len() + 4);
    for ch in id.chars() {
        if ch.is_uppercase() {
            kebab.push('-');
            kebab.extend(ch.to_lowercase());
        } else {
            kebab.push(ch);
        }
    }
    let kebab = kebab.strip_suffix("-landscape").or_else(|| kebab.strip_suffix("-portrait")).unwrap_or(&kebab).to_string();
    let kebab = alias(&kebab);
    if ACTIONS.iter().any(|a| a.id == kebab) {
        return kebab.to_string();
    }
    id.to_string()
}

/// Held shortcuts in these categories keep firing until released.
pub fn holdable(id: &str) -> bool {
    matches!(find(id).map(|a| a.cat), Some("Nudge" | "Size" | "Double & Halve"))
}

/// Half-based actions whose repeat behaviour follows the General "repeated commands" setting.
pub fn half_based(id: &str) -> bool {
    matches!(
        id,
        "left-half" | "right-half" | "top-half" | "bottom-half" | "top-left" | "top-right" | "bottom-left" | "bottom-right"
    )
}

/// Which horizontal direction a half-based action pushes toward, for "adjacent display" repeats.
pub fn side(id: &str) -> i32 {
    match id {
        "left-half" | "top-left" | "bottom-left" | "move-left" | "first-third" | "first-fourth" => -1,
        "right-half" | "top-right" | "bottom-right" | "move-right" | "last-third" | "last-fourth" => 1,
        _ => 0,
    }
}

/// The opposite-side twin, used when a repeat crosses onto the adjacent display.
pub fn mirror(id: &str) -> &'static str {
    match id {
        "left-half" => "right-half",
        "right-half" => "left-half",
        "top-left" => "top-right",
        "top-right" => "top-left",
        "bottom-left" => "bottom-right",
        "bottom-right" => "bottom-left",
        "move-left" => "move-right",
        "move-right" => "move-left",
        "first-third" => "last-third",
        "last-third" => "first-third",
        "first-fourth" => "last-fourth",
        "last-fourth" => "first-fourth",
        _ => "",
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gaps {
    /// Inset by half a gap between windows and a whole gap at screen edges.
    Tile,
    Maximize,
    MaximizeHeight,
    None,
}

pub struct Ctx<'a> {
    pub win: Rect,
    /// Usable area of the destination screen, before window gaps.
    pub area: Rect,
    pub repeat: usize,
    /// Size override for half-based actions when cycling sizes.
    pub frac: Option<f64>,
    pub portrait: bool,
    /// Other windows on the same screen, for Fill and Snap.
    pub others: &'a [Rect],
    pub t: &'a Tweaks,
}

pub struct Out {
    pub rect: Rect,
    pub gaps: Gaps,
}

fn tile(r: Rect) -> Option<Out> {
    Some(Out { rect: r, gaps: Gaps::Tile })
}
fn free(r: Rect) -> Option<Out> {
    Some(Out { rect: r, gaps: Gaps::None })
}

/// Cell (col,row) spanning (cs,rs) of a cols x rows grid.
fn grid(a: &Rect, cols: f64, rows: f64, col: f64, row: f64, cs: f64, rs: f64) -> Rect {
    a.sub(col / cols, row / rows, cs / cols, rs / rows)
}

/// Columns on a landscape screen, rows on a portrait one.
fn band(c: &Ctx, n: f64, i: f64, span: f64) -> Rect {
    if c.portrait {
        grid(&c.area, 1.0, n, 0.0, i, 1.0, span)
    } else {
        grid(&c.area, n, 1.0, i, 0.0, span, 1.0)
    }
}

fn cyc(start: usize, n: usize, rep: usize, fwd: bool) -> usize {
    if fwd { (start + rep) % n } else { (start + n - rep % n) % n }
}

pub fn calc(id: &str, c: &Ctx) -> Option<Out> {
    let a = c.area;
    let w = c.win;
    let f = c.frac.unwrap_or(0.5);
    let r = c.repeat;
    let t = c.t;
    match alias(id) {
        "maximize" => Some(Out { rect: a, gaps: Gaps::Maximize }),
        "almost-maximize" => {
            let (aw, ah) = (a.w * t.almost_maximize_width, a.h * t.almost_maximize_height);
            free(Rect::new(a.mid_x() - aw / 2.0, a.mid_y() - ah / 2.0, aw, ah))
        }
        "maximize-height" => Some(Out { rect: Rect::new(w.x, a.y, w.w.min(a.w), a.h), gaps: Gaps::MaximizeHeight }),
        "maximize-width" => Some(Out { rect: Rect::new(a.x, w.y, a.w, w.h.min(a.h)), gaps: Gaps::MaximizeHeight }),
        "left-half" => tile(a.sub(0.0, 0.0, f, 1.0)),
        "right-half" => tile(a.sub(1.0 - f, 0.0, f, 1.0)),
        "top-half" => tile(a.sub(0.0, 0.0, 1.0, f)),
        "bottom-half" => tile(a.sub(0.0, 1.0 - f, 1.0, f)),
        "reasonable-size" => free(a.sub(0.1, 0.1, 0.8, 0.8)),
        "center-half" => {
            let f = c.frac.unwrap_or(if t.center_half_cycles { [0.5, 2.0 / 3.0, 1.0 / 3.0][r % 3] } else { 0.5 });
            tile(a.sub((1.0 - f) / 2.0, 0.0, f, 1.0))
        }
        "top-left" => tile(a.sub(0.0, 0.0, f, 0.5)),
        "top-right" => tile(a.sub(1.0 - f, 0.0, f, 0.5)),
        "bottom-left" => tile(a.sub(0.0, 0.5, f, 0.5)),
        "bottom-right" => tile(a.sub(1.0 - f, 0.5, f, 0.5)),

        "first-third" => tile(band(c, 3.0, cyc(0, 3, r, true) as f64, 1.0)),
        "center-third" => tile(band(c, 3.0, 1.0, 1.0)),
        "last-third" => tile(band(c, 3.0, cyc(2, 3, r, false) as f64, 1.0)),
        "first-two-thirds" => tile(band(c, 3.0, if r % 2 == 0 { 0.0 } else { 1.0 }, 2.0)),
        "last-two-thirds" => tile(band(c, 3.0, if r % 2 == 0 { 1.0 } else { 0.0 }, 2.0)),
        "center-two-thirds" => tile(band(c, 6.0, 1.0, 4.0)),

        "top-third" => tile(grid(&a, 1.0, 3.0, 0.0, cyc(0, 3, r, true) as f64, 1.0, 1.0)),
        "vertical-center-third" => tile(grid(&a, 1.0, 3.0, 0.0, 1.0, 1.0, 1.0)),
        "bottom-third" => tile(grid(&a, 1.0, 3.0, 0.0, cyc(2, 3, r, false) as f64, 1.0, 1.0)),
        "top-two-thirds" => tile(grid(&a, 1.0, 3.0, 0.0, if r % 2 == 0 { 0.0 } else { 1.0 }, 1.0, 2.0)),
        "bottom-two-thirds" => tile(grid(&a, 1.0, 3.0, 0.0, if r % 2 == 0 { 1.0 } else { 0.0 }, 1.0, 2.0)),
        "center-horizontal-third" => tile(grid(&a, 3.0, 1.0, 1.0, 0.0, 1.0, 1.0)),
        "center-vertical-two-thirds" => tile(grid(&a, 1.0, 3.0, 0.0, 0.5, 1.0, 2.0)),
        "center-horizontal-two-thirds" => tile(grid(&a, 3.0, 1.0, 0.5, 0.0, 2.0, 1.0)),

        id @ ("top-left-third" | "top-right-third" | "bottom-right-third" | "bottom-left-third") => {
            // Clockwise cycle through the four corner two-thirds.
            let order = ["top-left-third", "top-right-third", "bottom-right-third", "bottom-left-third"];
            let start = order.iter().position(|o| *o == id).unwrap();
            let (cx, cy) = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)][cyc(start, 4, r, true)];
            tile(grid(&a, 3.0, 3.0, cx, cy, 2.0, 2.0))
        }

        "first-fourth" => tile(band(c, 4.0, cyc(0, 4, r, true) as f64, 1.0)),
        "second-fourth" => tile(band(c, 4.0, 1.0, 1.0)),
        "third-fourth" => tile(band(c, 4.0, 2.0, 1.0)),
        "last-fourth" => tile(band(c, 4.0, cyc(3, 4, r, false) as f64, 1.0)),
        "first-three-fourths" => tile(band(c, 4.0, if r % 2 == 0 { 0.0 } else { 1.0 }, 3.0)),
        "center-three-fourths" => tile(band(c, 8.0, 1.0, 6.0)),
        "last-three-fourths" => tile(band(c, 4.0, if r % 2 == 0 { 1.0 } else { 0.0 }, 3.0)),
        "top-fourth" => tile(grid(&a, 1.0, 4.0, 0.0, 0.0, 1.0, 1.0)),
        "center-top-fourth" => tile(grid(&a, 1.0, 4.0, 0.0, 1.0, 1.0, 1.0)),
        "center-bottom-fourth" => tile(grid(&a, 1.0, 4.0, 0.0, 2.0, 1.0, 1.0)),
        "bottom-fourth" => tile(grid(&a, 1.0, 4.0, 0.0, 3.0, 1.0, 1.0)),
        "top-three-fourths" => tile(grid(&a, 1.0, 4.0, 0.0, 0.0, 1.0, 3.0)),
        "bottom-three-fourths" => tile(grid(&a, 1.0, 4.0, 0.0, 1.0, 1.0, 3.0)),
        "center-vertical-three-fourths" => tile(grid(&a, 1.0, 8.0, 0.0, 1.0, 1.0, 6.0)),
        "center-horizontal-three-fourths" => tile(grid(&a, 8.0, 1.0, 1.0, 0.0, 6.0, 1.0)),
        "center-vertical-half" => tile(grid(&a, 1.0, 4.0, 0.0, 1.0, 1.0, 2.0)),

        "first-fifth" => tile(band(c, 5.0, cyc(0, 5, r, true) as f64, 1.0)),
        "second-fifth" => tile(band(c, 5.0, 1.0, 1.0)),
        "third-fifth" => tile(band(c, 5.0, 2.0, 1.0)),
        "fourth-fifth" => tile(band(c, 5.0, 3.0, 1.0)),
        "last-fifth" => tile(band(c, 5.0, cyc(4, 5, r, false) as f64, 1.0)),

        "first-sixth" => tile(band(c, 6.0, cyc(0, 6, r, true) as f64, 1.0)),
        "second-sixth" => tile(band(c, 6.0, 1.0, 1.0)),
        "third-sixth" => tile(band(c, 6.0, 2.0, 1.0)),
        "fourth-sixth" => tile(band(c, 6.0, 3.0, 1.0)),
        "fifth-sixth" => tile(band(c, 6.0, 4.0, 1.0)),
        "last-sixth" => tile(band(c, 6.0, cyc(5, 6, r, false) as f64, 1.0)),
        "left-center-sixth" => tile(grid(&a, 2.0, 3.0, 0.0, 1.0, 1.0, 1.0)),
        "right-center-sixth" => tile(grid(&a, 2.0, 3.0, 1.0, 1.0, 1.0, 1.0)),
        id @ ("top-left-two-sixths" | "top-right-two-sixths" | "bottom-left-two-sixths" | "bottom-right-two-sixths") => {
            let (cx, cy) = match id {
                "top-left-two-sixths" => (0.0, 0.0),
                "top-right-two-sixths" => (1.0, 0.0),
                "bottom-left-two-sixths" => (0.0, 1.0),
                _ => (1.0, 1.0),
            };
            tile(grid(&a, 3.0, 2.0, cx, cy, 2.0, 1.0))
        }

        id if id.ends_with("-sixth") || id.ends_with("-eighth") || id.ends_with("-ninth") => {
            let (names, cols, rows): (&[&str], usize, usize) = if id.ends_with("-sixth") {
                (
                    if c.portrait {
                        &["top-left-sixth", "top-right-sixth", "left-center-sixth", "right-center-sixth", "bottom-left-sixth", "bottom-right-sixth"]
                    } else {
                        &["top-left-sixth", "top-center-sixth", "top-right-sixth", "bottom-left-sixth", "bottom-center-sixth", "bottom-right-sixth"]
                    },
                    if c.portrait { 2 } else { 3 },
                    if c.portrait { 3 } else { 2 },
                )
            } else if id.ends_with("-eighth") {
                (
                    &[
                        "top-left-eighth", "top-center-left-eighth", "top-center-right-eighth", "top-right-eighth",
                        "bottom-left-eighth", "bottom-center-left-eighth", "bottom-center-right-eighth", "bottom-right-eighth",
                    ],
                    4,
                    2,
                )
            } else {
                (
                    &[
                        "top-left-ninth", "top-center-ninth", "top-right-ninth", "middle-left-ninth", "middle-center-ninth",
                        "middle-right-ninth", "bottom-left-ninth", "bottom-center-ninth", "bottom-right-ninth",
                    ],
                    3,
                    3,
                )
            };
            let start = names.iter().position(|n| *n == id)?;
            let i = cyc(start, names.len(), r, true);
            // Landscape order is row-major; a portrait sixths grid is 2 wide, read the same way.
            let (col, row) = (i % cols, i / cols);
            tile(grid(&a, cols as f64, rows as f64, col as f64, row as f64, 1.0, 1.0))
        }

        dir @ ("move-left" | "move-right" | "move-up" | "move-down") => {
            let mut n = w;
            n.w = n.w.min(a.w);
            n.h = n.h.min(a.h);
            if t.resize_on_directional_move {
                let fr = [0.5, 2.0 / 3.0, 1.0 / 3.0][r % 3];
                if dir == "move-left" || dir == "move-right" {
                    n.w = a.w * fr;
                } else {
                    n.h = a.h * fr;
                }
            }
            match dir {
                "move-left" | "move-right" => {
                    n.x = if dir == "move-left" { a.x } else { a.max_x() - n.w };
                    if t.centered_directional_move {
                        n.y = a.mid_y() - n.h / 2.0;
                    }
                }
                _ => {
                    n.y = if dir == "move-up" { a.y } else { a.max_y() - n.h };
                    if t.centered_directional_move {
                        n.x = a.mid_x() - n.w / 2.0;
                    }
                }
            }
            free(n.clamped_into(&a))
        }

        id if id.starts_with("snap-") => {
            let (l, rt, up, dn) = (id.contains("left"), id.contains("right"), id.contains("up") || id.contains("top"), id.contains("down") || id.contains("bottom"));
            let mut n = w;
            if l {
                let edge = c.others.iter().filter(|o| o.max_y() > w.y && o.y < w.max_y() && o.max_x() <= w.x + 1.0).map(|o| o.max_x()).fold(a.x, f64::max);
                n.x = edge;
            }
            if rt {
                let edge = c.others.iter().filter(|o| o.max_y() > w.y && o.y < w.max_y() && o.x >= w.max_x() - 1.0).map(|o| o.x).fold(a.max_x(), f64::min);
                n.x = edge - w.w;
            }
            if up {
                let edge = c.others.iter().filter(|o| o.max_x() > n.x && o.x < n.max_x() && o.max_y() <= w.y + 1.0).map(|o| o.max_y()).fold(a.y, f64::max);
                n.y = edge;
            }
            if dn {
                let edge = c.others.iter().filter(|o| o.max_x() > n.x && o.x < n.max_x() && o.y >= w.max_y() - 1.0).map(|o| o.y).fold(a.max_y(), f64::min);
                n.y = edge - w.h;
            }
            free(n)
        }

        "nudge-left" => free(Rect { x: w.x - t.nudge_offset, ..w }),
        "nudge-right" => free(Rect { x: w.x + t.nudge_offset, ..w }),
        "nudge-up" => free(Rect { y: w.y - t.nudge_offset, ..w }),
        "nudge-down" => free(Rect { y: w.y + t.nudge_offset, ..w }),

        id @ ("larger" | "smaller" | "larger-width" | "smaller-width" | "larger-height" | "smaller-height") => {
            let grow = id.starts_with("larger");
            let horiz = !id.ends_with("-height");
            let vert = !id.ends_with("-width");
            free(resize(&w, &a, grow, horiz, vert, t))
        }

        "double-width-right" => free(Rect { w: (w.w * 2.0).min(a.max_x() - w.x), ..w }),
        "double-width-left" => {
            let nx = (w.max_x() - w.w * 2.0).max(a.x);
            free(Rect::new(nx, w.y, w.max_x() - nx, w.h))
        }
        "double-height-down" => free(Rect { h: (w.h * 2.0).min(a.max_y() - w.y), ..w }),
        "double-height-up" => {
            let ny = (w.max_y() - w.h * 2.0).max(a.y);
            free(Rect::new(w.x, ny, w.w, w.max_y() - ny))
        }
        "halve-width-left" => free(Rect { w: w.w / 2.0, ..w }),
        "halve-width-right" => free(Rect::new(w.x + w.w / 2.0, w.y, w.w / 2.0, w.h)),
        "halve-height-up" => free(Rect { h: w.h / 2.0, ..w }),
        "halve-height-down" => free(Rect::new(w.x, w.y + w.h / 2.0, w.w, w.h / 2.0)),

        "center" => free(w.centered_in(&a)),
        "upper-center" => {
            let n = w.centered_in(&a);
            free(Rect { y: a.y + (a.h - n.h) / 3.0, ..n })
        }
        "specified" => free(Rect::new(0.0, 0.0, t.specified_width, t.specified_height).centered_in(&a)),

        id if id.starts_with("fill-") => fill(id, c),
        _ => None,
    }
}

/// Larger/Smaller. With curtain resize, edges touching the screen edge stay put.
fn resize(w: &Rect, a: &Rect, grow: bool, horiz: bool, vert: bool, t: &Tweaks) -> Rect {
    let d = if grow { t.size_offset } else { -t.size_offset };
    let tol = 2.0;
    let pin = |touch: bool| t.curtain_resize && touch;
    let (mut l, mut r, mut top, mut bot) = (w.x, w.max_x(), w.y, w.max_y());
    if horiz {
        let (lt, rt) = (pin((w.x - a.x).abs() < tol), pin((w.max_x() - a.max_x()).abs() < tol));
        match (lt, rt) {
            (true, false) => r += d * 2.0,
            (false, true) => l -= d * 2.0,
            (true, true) if !grow => r += d * 2.0,
            (true, true) => {}
            _ => {
                l -= d;
                r += d;
            }
        }
    }
    if vert {
        let (tt, bt) = (pin((w.y - a.y).abs() < tol), pin((w.max_y() - a.max_y()).abs() < tol));
        match (tt, bt) {
            (true, false) => bot += d * 2.0,
            (false, true) => top -= d * 2.0,
            (true, true) if !grow => bot += d * 2.0,
            (true, true) => {}
            _ => {
                top -= d;
                bot += d;
            }
        }
    }
    let mut n = Rect::new(l, top, r - l, bot - top);
    if grow {
        n = n.intersect(a);
        if n.area() == 0.0 {
            return *w;
        }
    } else {
        let (minw, minh) = (a.w * t.minimum_window_width, a.h * t.minimum_window_height);
        if horiz && n.w < minw {
            n.x = w.x;
            n.w = w.w.min(minw.max(w.w));
        }
        if vert && n.h < minh {
            n.y = w.y;
            n.h = w.h.min(minh.max(w.h));
        }
    }
    n
}

/// Maximal empty rectangles of `area` not covered by `obs`, from the obstacle edge lines.
pub fn empty_rects(area: &Rect, obs: &[Rect]) -> Vec<Rect> {
    let obs: Vec<Rect> = obs.iter().map(|o| o.intersect(area)).filter(|o| o.area() > 0.0).take(24).collect();
    let mut xs = vec![area.x, area.max_x()];
    let mut ys = vec![area.y, area.max_y()];
    for o in &obs {
        xs.extend([o.x, o.max_x()]);
        ys.extend([o.y, o.max_y()]);
    }
    let dedup = |v: &mut Vec<f64>| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v.dedup_by(|a, b| (*a - *b).abs() < 1.0);
    };
    dedup(&mut xs);
    dedup(&mut ys);
    let mut out: Vec<Rect> = Vec::new();
    for (i, &x0) in xs.iter().enumerate() {
        for &x1 in &xs[i + 1..] {
            for (j, &y0) in ys.iter().enumerate() {
                for &y1 in &ys[j + 1..] {
                    let r = Rect::new(x0, y0, x1 - x0, y1 - y0);
                    if obs.iter().any(|o| o.intersect(&r).area() > 1.0) {
                        break; // taller rects from this y0 also hit the obstacle
                    }
                    out.push(r);
                }
            }
        }
    }
    // Keep only maximal ones.
    let maximal: Vec<Rect> = out
        .iter()
        .filter(|r| {
            !out.iter().any(|o| {
                o != *r && o.x <= r.x + 0.5 && o.y <= r.y + 0.5 && o.max_x() >= r.max_x() - 0.5 && o.max_y() >= r.max_y() - 0.5
            })
        })
        .copied()
        .collect();
    maximal
}

fn fill(id: &str, c: &Ctx) -> Option<Out> {
    let a = c.area;
    let (minw, minh) = (a.w * 0.12, a.h * 0.25);
    let mut cands: Vec<Rect> = empty_rects(&a, c.others)
        .into_iter()
        .filter(|r| r.w >= minw && r.h >= minh)
        .filter(|r| match id {
            "fill-left" => r.mid_x() < a.mid_x() - 1.0,
            "fill-right" => r.mid_x() > a.mid_x() + 1.0,
            "fill-top-left" => r.mid_x() < a.mid_x() - 1.0 && r.mid_y() < a.mid_y() - 1.0,
            "fill-top-right" => r.mid_x() > a.mid_x() + 1.0 && r.mid_y() < a.mid_y() - 1.0,
            "fill-bottom-left" => r.mid_x() < a.mid_x() - 1.0 && r.mid_y() > a.mid_y() + 1.0,
            "fill-bottom-right" => r.mid_x() > a.mid_x() + 1.0 && r.mid_y() > a.mid_y() + 1.0,
            _ => false,
        })
        .collect();
    cands.sort_by(|x, y| y.area().partial_cmp(&x.area()).unwrap());
    if cands.is_empty() {
        let fallback = match id {
            "fill-left" => "left-half",
            "fill-right" => "right-half",
            "fill-top-left" => "top-left",
            "fill-top-right" => "top-right",
            "fill-bottom-left" => "bottom-left",
            _ => "bottom-right",
        };
        return calc(fallback, &Ctx { repeat: 0, frac: None, ..*c });
    }
    tile(cands[c.repeat % cands.len()])
}

/// Where an action lands on a 1x1 screen (for icons), or None if it depends on the window.
pub fn unit_preview(id: &str) -> Option<Rect> {
    let t = Tweaks::default();
    let area = Rect::new(0.0, 0.0, 160.0, 100.0);
    let win = Rect::new(45.0, 30.0, 70.0, 40.0);
    let ctx = Ctx { win, area, repeat: 0, frac: None, portrait: false, others: &[], t: &t };
    let out = calc(id, &ctx)?;
    Some(area.to_unit(&out.rect))
}

/// Apply window gaps: a full gap at screen edges (when enabled), half a gap between windows.
pub fn apply_gaps(r: Rect, area: &Rect, gap: f64, on_edges: bool) -> Rect {
    if gap <= 0.0 {
        return r;
    }
    let tol = 1.5;
    let e = |touch: bool| if touch { if on_edges { gap } else { 0.0 } } else { gap / 2.0 };
    r.inset(
        e((r.x - area.x).abs() < tol),
        e((r.y - area.y).abs() < tol),
        e((r.max_x() - area.max_x()).abs() < tol),
        e((r.max_y() - area.max_y()).abs() < tol),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(t: &'a Tweaks, others: &'a [Rect], repeat: usize) -> Ctx<'a> {
        Ctx {
            win: Rect::new(100.0, 100.0, 400.0, 300.0),
            area: Rect::new(0.0, 25.0, 1200.0, 800.0),
            repeat,
            frac: None,
            portrait: false,
            others,
            t,
        }
    }

    #[test]
    fn halves_and_thirds() {
        let t = Tweaks::default();
        let c = ctx(&t, &[], 0);
        assert_eq!(calc("left-half", &c).unwrap().rect, Rect::new(0.0, 25.0, 600.0, 800.0));
        assert_eq!(calc("last-third", &c).unwrap().rect, Rect::new(800.0, 25.0, 400.0, 800.0));
        let c1 = ctx(&t, &[], 1);
        assert_eq!(calc("first-third", &c1).unwrap().rect, Rect::new(400.0, 25.0, 400.0, 800.0));
        assert_eq!(calc("last-third", &c1).unwrap().rect, Rect::new(400.0, 25.0, 400.0, 800.0));
    }

    #[test]
    fn every_catalog_action_is_handled_or_engine_owned() {
        let engine = [
            "Display", "Control", "Multiple Windows", "Stash", "Pin", "App",
        ];
        let t = Tweaks::default();
        let c = ctx(&t, &[], 0);
        for a in ACTIONS {
            if engine.contains(&a.cat) || matches!(a.id, "restore" | "last") {
                continue;
            }
            assert!(calc(a.id, &c).is_some(), "{} has no geometry", a.id);
        }
    }

    #[test]
    fn ninths_cycle() {
        let t = Tweaks::default();
        let c = ctx(&t, &[], 4);
        let r = calc("top-left-ninth", &c).unwrap().rect;
        assert!(r.approx_eq(&Rect::new(400.0, 25.0 + 800.0 / 3.0, 400.0, 800.0 / 3.0), 0.01), "{r:?}");
    }

    #[test]
    fn fill_uses_empty_space() {
        let t = Tweaks::default();
        let others = [Rect::new(500.0, 25.0, 700.0, 800.0)];
        let c = ctx(&t, &others, 0);
        assert_eq!(calc("fill-left", &c).unwrap().rect, Rect::new(0.0, 25.0, 500.0, 800.0));
        let c = ctx(&t, &[], 0);
        assert_eq!(calc("fill-right", &c).unwrap().rect, Rect::new(600.0, 25.0, 600.0, 800.0));
    }

    #[test]
    fn gaps() {
        let a = Rect::new(0.0, 0.0, 1000.0, 1000.0);
        let r = apply_gaps(a.sub(0.0, 0.0, 0.5, 1.0), &a, 10.0, true);
        assert_eq!(r, Rect::new(10.0, 10.0, 485.0, 980.0));
    }

    #[test]
    fn smaller_respects_curtain() {
        let t = Tweaks::default();
        let mut c = ctx(&t, &[], 0);
        c.win = Rect::new(0.0, 25.0, 600.0, 800.0);
        let r = calc("smaller", &c).unwrap().rect;
        assert_eq!(r.x, 0.0);
        assert_eq!(r.w, 540.0);
    }
}
