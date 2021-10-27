use crate::{
    app_info::{engine_version, AppInfo},
    Core,
};
use anyhow::Result;
use erupt::{
    cstr,
    vk, DeviceLoader, EntryLoader, InstanceLoader,
};
use gpu_alloc::GpuAllocator;
use std::ffi::CString;
use std::sync::Mutex;
use std::{ffi::CStr, os::raw::c_char};

pub fn build_core(info: AppInfo) -> Result<Core> {
    // Entry
    let entry = EntryLoader::new()?;

    // Instance
    let app_name = CString::new(info.name)?;
    let engine_name = CString::new(crate::ENGINE_NAME)?;
    let app_info = vk::ApplicationInfoBuilder::new()
        .application_name(&app_name)
        .application_version(info.version)
        .engine_name(&engine_name)
        .engine_version(engine_version())
        .api_version(info.api_version);

    // Instance and device layers and extensions
    let mut instance_layers = Vec::new();
    let mut instance_extensions = vec![];
    let mut device_layers = Vec::new();
    let device_extensions = vec![];

    if info.validation {
        const LAYER_KHRONOS_VALIDATION: *const i8 = cstr!("VK_LAYER_KHRONOS_validation");
        instance_extensions
            .push(erupt::extensions::ext_debug_utils::EXT_DEBUG_UTILS_EXTENSION_NAME);
        instance_layers.push(LAYER_KHRONOS_VALIDATION);
        device_layers.push(LAYER_KHRONOS_VALIDATION);
    }

    // Instance creation
    let create_info = vk::InstanceCreateInfoBuilder::new()
        .application_info(&app_info)
        .enabled_extension_names(&instance_extensions)
        .enabled_layer_names(&instance_layers);

    let instance = InstanceLoader::new(&entry, &create_info, None)?;

    // Hardware selection
    let hardware = HeadlessHardwareSelection::query(&instance, &device_extensions)?;

    // Create logical device and queues
    let create_info = [vk::DeviceQueueCreateInfoBuilder::new()
        .queue_family_index(hardware.queue_family)
        .queue_priorities(&[1.0])];

    let physical_device_features = vk::PhysicalDeviceFeaturesBuilder::new();
    let create_info = vk::DeviceCreateInfoBuilder::new()
        .queue_create_infos(&create_info)
        .enabled_features(&physical_device_features)
        .enabled_extension_names(&device_extensions)
        .enabled_layer_names(&device_layers);

    let device = DeviceLoader::new(&instance, hardware.physical_device, &create_info, None)?;
    let queue = unsafe { device.get_device_queue(hardware.queue_family, 0, None) };

    let device_props =
        unsafe { gpu_alloc_erupt::device_properties(&instance, hardware.physical_device)? };
    let allocator = Mutex::new(GpuAllocator::new(
        gpu_alloc::Config::i_am_prototyping(), // TODO: SET THIS TO SOMETHING MORE SANE!! Maybe embed in AppInfo?!
        device_props,
    ));
    let device_properties =
        unsafe { instance.get_physical_device_properties(hardware.physical_device, None) };

    Ok(Core {
        physical_device: hardware.physical_device,
        device_properties,
        queue_family: hardware.queue_family,
        queue,
        device,
        instance,
        allocator,
        entry,
    })
}

pub struct HeadlessHardwareSelection {
    pub physical_device: vk::PhysicalDevice,
    pub physical_device_properties: vk::PhysicalDeviceProperties,
    pub queue_family: u32,
}

impl HeadlessHardwareSelection {
    pub fn query(
        instance: &InstanceLoader,
        device_extensions: &[*const c_char],
    ) -> Result<Self> {
        unsafe { instance.enumerate_physical_devices(None) }
        .unwrap()
            .into_iter()
            .filter_map(|physical_device| unsafe {
                let queue_family = match instance
                    .get_physical_device_queue_family_properties(physical_device, None)
                    .into_iter()
                    .position(|properties| properties.queue_flags.contains(vk::QueueFlags::GRAPHICS)) 
                    {
                        Some(queue_family) => queue_family as u32,
                        None => return None,
                    };

                let supported_extensions = instance
                    .enumerate_device_extension_properties(physical_device, None, None)
                    .unwrap();
                if !device_extensions.iter().all(|device_extension| {
                    let device_extension = CStr::from_ptr(*device_extension);

                    supported_extensions.iter().any(|properties| {
                        CStr::from_ptr(properties.extension_name.as_ptr()) == device_extension
                    })
                }) {
                    return None;
                }

                let physical_device_properties =
                    instance.get_physical_device_properties(physical_device, None);
                Some(Self {
                    physical_device,
                    queue_family,
                    physical_device_properties,
                })
            })
        .max_by_key(|query| match query.physical_device_properties.device_type {
            vk::PhysicalDeviceType::DISCRETE_GPU => 2,
            vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
            _ => 0,
        })
        .ok_or_else(|| anyhow::format_err!("No suitable hardware found for this configuration"))
    }
}
