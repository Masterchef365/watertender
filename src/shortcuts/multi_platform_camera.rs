use crate::{Platform, PlatformEvent, PlatformReturn};
use anyhow::Result;

pub struct MultiPlatformCamera;

impl MultiPlatformCamera {
    pub fn new(platform: Platform<'_>) -> Self {
        Self
    }

    /*
    pub fn get_matrices(&self, platform: Platform) -> (PlatformReturn, [f32; 4*4*2]) {
        [0.; 4*4*2]
    }
    */

    pub fn handle_event(&mut self, event: &mut PlatformEvent<'_, '_>, platform: &mut Platform<'_>) {

    }
}
