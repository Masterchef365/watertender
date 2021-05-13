//! Sensible defaults used elsewhere in the Shortcut API. May also be used by the mainloop
//! abstraction.
use erupt::{vk, extensions::khr_surface::ColorSpaceKHR};

/// Decent depth format
pub const DEPTH_FORMAT: vk::Format = vk::Format::D32_SFLOAT; // TODO: Add stencil? Check compat...

/// Decent color format
pub const COLOR_FORMAT: vk::Format = vk::Format::B8G8R8A8_SRGB;

/// Used in shortcuts, to make things easier
pub const COLOR_SPACE: ColorSpaceKHR = ColorSpaceKHR::SRGB_NONLINEAR_KHR;

