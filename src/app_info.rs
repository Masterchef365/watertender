use anyhow::Result;
use erupt::vk;

/// Application info
pub struct AppInfo {
    pub(crate) name: String,
    pub(crate) version: u32,
    pub(crate) api_version: u32,
    pub(crate) validation: bool,
}

// TODO: Device extensions!
impl AppInfo {
    pub fn app_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.version = vk::make_version(major, minor, patch);
        self
    }

    pub fn vk_version(mut self, major: u32, minor: u32, patch: u32) -> Self {
        self.api_version = vk::make_version(major, minor, patch);
        self
    }

    pub fn name(mut self, name: String) -> Result<Self> {
        self.name = name;
        Ok(self)
    }

    pub fn validation(mut self, validation: bool) -> Self {
        self.validation = validation;
        self
    }
}

impl Default for AppInfo {
    /// Defaults to Vulkan 1.1, with validation layers disabled.
    fn default() -> Self {
        Self {
            name: env!("CARGO_PKG_NAME").to_owned(),
            api_version: vk::make_version(1, 1, 0),
            version: vk::make_version(1, 0, 0),
            validation: false,
        }
    }
}

/// This crate's version as a Vulkan-formatted u32. Note that this requires `vk` to be in the
/// current namespace.
#[macro_export]
macro_rules! cargo_vk_version {
    () => {
        vk::make_version(
            env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
        )
    };
}

/// Return the Vulkan-ready version of this engine
pub fn engine_version() -> u32 {
    cargo_vk_version!()
}
