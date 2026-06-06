mod component;
mod judgement_circle;
mod movement;
pub mod resources;
mod shapes;
mod slide_path;
mod spawning;

pub use judgement_circle::spawn_judgement_ring;
pub use movement::update_movement;
pub use spawning::next_event;

const RADIUS: f32 = 350.0;
const NOTE_RADIUS: f32 = 35.0;
/// Spacing between chevron arrows along a slide track. Tune visually.
const CHEVRON_SPACING: f32 = NOTE_RADIUS * 0.75;

#[allow(unused)]
pub mod note_colors {
    use bevy::color::Color;

    // ── Core note types ────────────────────────────────────────────────

    /// Tap and hold notes (pink).
    pub const TAP: Color = Color::srgb(1.0, 0.4, 0.6);

    /// Hold notes share the same tint as tap.
    pub const HOLD: Color = TAP;

    /// Slide notes and touch-note centre (blue).
    pub const SLIDE: Color = Color::srgb(0.4, 0.8, 1.0);

    /// Touch note centre dot reuses the slide color.
    pub const TOUCH: Color = SLIDE;

    /// Paired (multi-note) overlay color (yellow).
    pub const PAIRED: Color = Color::srgb(1.0, 1.0, 0.0);

    // ── Touch-hold directional triangles ───────────────────────────────

    /// Top triangle (red).
    pub const TOUCH_HOLD_TOP: Color = Color::srgb(1.0, 0.0, 0.0);

    /// Bottom triangle (yellow — same hue as paired, but a distinct role).
    pub const TOUCH_HOLD_BOTTOM: Color = Color::srgb(1.0, 1.0, 0.0);

    /// Left triangle (green).
    pub const TOUCH_HOLD_LEFT: Color = Color::srgb(0.0, 1.0, 0.0);

    /// Right triangle (blue).
    pub const TOUCH_HOLD_RIGHT: Color = Color::srgb(0.0, 0.0, 1.0);

    /// All four directional colours in spawn order
    /// (top, bottom, left, right — matching `[Y, -Y, -X, X]`).
    pub const TOUCH_HOLD_DIRS: [Color; 4] = [
        TOUCH_HOLD_TOP,
        TOUCH_HOLD_BOTTOM,
        TOUCH_HOLD_LEFT,
        TOUCH_HOLD_RIGHT,
    ];

    // ── UI / environmental ─────────────────────────────────────────────

    /// Judgement ring and countdown arc stroke.
    pub const RING: Color = Color::WHITE;

    /// Slide track chevron tint (light cyan).
    /// Matches `bevy::color::palettes::css::LIGHT_CYAN` but kept here so
    /// every color lives in one file.
    pub const CHEVRON: Color = Color::srgb(0.878, 1.0, 1.0);

    /// Screen background.
    pub const BACKGROUND: Color = Color::srgb(0.0, 0.0, 0.0);
}
