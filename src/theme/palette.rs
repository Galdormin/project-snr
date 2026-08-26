use bevy::prelude::*;

/// #070b12
pub const BACKGROUND_COLOR: Color = Color::srgb_u8(7, 11, 18);

/// #5ef0c8
pub const PHOSPHORE_HIGH: Color = Color::srgb_u8(94, 240, 200);
/// #4d9887
pub const PHOSPHORE_MID: Color = Color::srgb_u8(77, 152, 135);
/// #2e7a68
pub const PHOSPHORE_LOW: Color = Color::srgb_u8(46, 122, 104);

// Gameplay readout colors. These are the colors a sonar point can be painted
// with, and are the only place [`PHOSPHORE_HIGH`] is meant to appear.

/// #e8564c
pub const RED_HIGH: Color = Color::srgb_u8(232, 86, 76);
/// #5eb4f0
pub const AZURE_HIGH: Color = Color::srgb_u8(94, 180, 240);
/// #f0b45e
pub const AMBER_HIGH: Color = Color::srgb_u8(240, 180, 94);
/// #c88ef0
pub const VIOLET_HIGH: Color = Color::srgb_u8(200, 142, 240);
/// #e8e4d8
pub const BONE_HIGH: Color = Color::srgb_u8(232, 228, 216);

// In-game HUD chrome. Deliberately kept a notch below [`PHOSPHORE_HIGH`], which
// is reserved for gameplay readouts, so the HUD never competes with the sonar.

/// #071815, mostly opaque so the HUD stays readable over a bright scan.
pub const HUD_BACKGROUND: Color = Color::srgba_u8(7, 24, 21, 235);
/// #0f2b26
pub const HUD_BACKGROUND_HOVERED: Color = Color::srgba_u8(15, 43, 38, 245);
/// Slightly inset from [`HUD_BACKGROUND`], to read as an editable field.
pub const HUD_FIELD_BACKGROUND: Color = Color::srgba_u8(4, 15, 13, 245);
pub const HUD_BORDER: Color = PHOSPHORE_LOW;
pub const HUD_ACCENT: Color = PHOSPHORE_MID;
pub const HUD_TEXT: Color = PHOSPHORE_MID;
pub const HUD_TEXT_DIM: Color = PHOSPHORE_LOW;

pub const HEADER_TEXT: Color = PHOSPHORE_HIGH;
pub const LABEL_TEXT: Color = PHOSPHORE_HIGH;
pub const BUTTON_TEXT: Color = PHOSPHORE_HIGH;

/// #2e7a68
pub const BUTTON_BACKGROUND: Color = PHOSPHORE_LOW;

/// #4d9887
pub const BUTTON_HOVERED_BACKGROUND: Color = PHOSPHORE_MID;
pub const BUTTON_PRESSED_BACKGROUND: Color = BUTTON_HOVERED_BACKGROUND;
