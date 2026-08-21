use bevy::prelude::*;

/// #070b12
pub const BACKGROUND_COLOR: Color = Color::srgb_u8(7, 11, 18);

/// #5ef0c8
pub const PHOSPHORE_HIGH: Color = Color::srgb_u8(94, 240, 200);
/// #2e7a68
pub const PHOSPHORE_LOW: Color = Color::srgb_u8(46, 122, 104);

pub const HEADER_TEXT: Color = PHOSPHORE_HIGH;
pub const LABEL_TEXT: Color = PHOSPHORE_HIGH;
pub const BUTTON_TEXT: Color = PHOSPHORE_HIGH;

/// #2e7a68
pub const BUTTON_BACKGROUND: Color = PHOSPHORE_LOW;

/// #4d9887
pub const BUTTON_HOVERED_BACKGROUND: Color = Color::srgb_u8(77, 152, 135);
pub const BUTTON_PRESSED_BACKGROUND: Color = BUTTON_HOVERED_BACKGROUND;
