mod component;
mod components;
mod coord_type;
mod input_system;
mod layout_utils;
mod markers;
mod num_utils;
mod point;
mod rect;
mod rendering;
mod style;

pub use component::{
    component::{Component, ComponentType},
    component_state::{ComponentState, ComponentStateTrait},
    component_trait::ComponentTrait,
};
pub use components::{
    label::{Label, TextAlignment},
    pane::Pane,
    window_utils::{
        resize::ResizeHandle,
        window::{Window, WindowBuilder},
    },
};
pub use coord_type::CoordType;
pub use input_system::{
    input_device::InputDevice,
    input_event::InputEvent,
    key_event::{KeyCode, KeyEvent},
    key_modifiers::KeyModifiers,
    mouse_event::{MouseButton, MouseEvent, MouseEventKind, MouseScrollDirection},
    resize_event::ResizeEvent,
};
pub use layout_utils::{
    flex::{Direction, FlexItem, FlexLayout},
    grid::GridLayout,
    layout::{Layout, LayoutType},
    size_constraints::SizeConstraints,
};
pub use markers::{ComponentRequirements, ComponentTraitRequirements};
pub use num_utils::NumUtils;
pub use point::Point;
pub use rect::Rect;
pub use rendering::{
    flush_mode::FlushMode,
    glyph::{
        CompositeMetrics, GlyphId, GlyphMetrics, MonospaceMetrics, PositionedGlyph,
        ProportionalMetrics, FIRST_CUSTOM, MAX_CODEPOINT,
    },
    metrics::RendererMetrics,
    renderer::{Renderer, RendererError},
};
pub use style::{
    color::{AnsiColor, Color},
    modifier::Modifier,
    style::Style,
};
