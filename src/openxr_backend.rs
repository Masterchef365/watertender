use openxr as xr;
//use std::sync::Arc;

/// A container for several commonly-used OpenXR constants.
pub struct XrCore {
    pub instance: xr::Instance,
    pub session: xr::Session<xr::Vulkan>,
    pub system: xr::SystemId,
}

//pub type SharedXrCore = Arc<XrCore>;
