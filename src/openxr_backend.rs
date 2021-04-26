use crate::{AppInfo, MainLoop};
use anyhow::Result;
use openxr as xr;
//use std::sync::Arc;

/// A container for several commonly-used OpenXR constants.
pub struct XrCore {
    pub instance: xr::Instance,
    pub session: xr::Session<xr::Vulkan>,
    pub system: xr::SystemId,
}

//pub type SharedXrCore = Arc<XrCore>;

pub fn launch<M: MainLoop>(_info: AppInfo) -> Result<()> {
    todo!()
}
