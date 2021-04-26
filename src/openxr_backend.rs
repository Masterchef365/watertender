use erupt::{vk, EntryLoader, cstr, InstanceLoader};
use crate::{AppInfo, Core, MainLoop, Platform, PlatformEvent, SharedCore};
use anyhow::{bail, ensure, Result, Context};
use openxr as xr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use std::ffi::{CString, CStr};

pub type SharedXrCore = Arc<XrCore>;

/// A container for several commonly-used OpenXR constants.
pub struct XrCore {
    pub instance: xr::Instance,
    pub session: xr::Session<xr::Vulkan>,
    pub system: xr::SystemId,
}

/// Launch an `App` using OpenXR as a surface and input mechanism for VR
pub fn launch<M: MainLoop>(info: AppInfo) -> Result<()> {
    // Handle interrupts gracefully
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    })
    .expect("setting Ctrl-C handler");

    let (core, xr_core) = build_cores(info)?;
    let mut app = M::new(&core, Platform::OpenXr { xr_core: &xr_core })?;

    let mut event_storage = xr::EventDataBuffer::new();
    let mut session_running = false;

    // TODO: STATE TRANSITIONS
    'main_loop: loop {
        if !running.load(Ordering::Relaxed) {
            println!("Requesting exit");
            let res = xr_core.session.request_exit();
            if let Err(xr::sys::Result::ERROR_SESSION_NOT_RUNNING) = res {
                println!("OpenXR Exiting gracefully");
                break Ok(());
            }
            res?;
        }

        while let Some(event) = xr_core.instance.poll_event(&mut event_storage).unwrap() {
            use xr::Event::*;
            match event {
                SessionStateChanged(e) => {
                    println!("OpenXR entered state {:?}", e.state());
                    match e.state() {
                        xr::SessionState::READY => {
                            xr_core
                                .session
                                .begin(xr::ViewConfigurationType::PRIMARY_STEREO)
                                .unwrap();
                            session_running = true;
                        }
                        xr::SessionState::STOPPING => {
                            xr_core.session.end().unwrap();
                            session_running = false;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            println!("OpenXR Exiting");
                            break 'main_loop Ok(());
                        }
                        _ => {}
                    }
                }
                InstanceLossPending(_) => {
                    println!("OpenXR Pending instance loss");
                    break 'main_loop Ok(());
                }
                EventsLost(e) => {
                    println!("OpenXR lost {} events", e.lost_event_count());
                }
                _ => {}
            }
            app.event(
                PlatformEvent::OpenXr(&event),
                &core,
                Platform::OpenXr { xr_core: &xr_core },
            )?;
        }

        if !session_running {
            // Don't grind up the CPU
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }

        let swapchain_index = todo!();
        app.frame(
            crate::Frame { swapchain_index },
            &core,
            Platform::OpenXr { xr_core: &xr_core },
        )?;
    }
}

fn build_cores(info: AppInfo) -> Result<(SharedCore, SharedXrCore)> {
    // Load OpenXR runtime
    let xr_entry = xr::Entry::load()?;

    let available_extensions = xr_entry.enumerate_extensions()?;
    ensure!(
        available_extensions.khr_vulkan_enable2,
        "Klystron requires OpenXR with KHR_VULKAN_ENABLE2"
    );

    let mut enabled_extensions = xr::ExtensionSet::default();
    enabled_extensions.khr_vulkan_enable2 = true;

    let xr_instance = xr_entry.create_instance(
        &xr::ApplicationInfo {
            application_name: &info.name,
            application_version: info.version,
            engine_name: crate::ENGINE_NAME,
            engine_version: crate::engine_version(),
        },
        &enabled_extensions,
        &[],
    )?;
    let instance_props = xr_instance.properties()?;

    println!(
        "Loaded OpenXR runtime: {} {}",
        instance_props.runtime_name, instance_props.runtime_version
    );

    let system = xr_instance
        .system(xr::FormFactor::HEAD_MOUNTED_DISPLAY)
        .unwrap();

    // Load Vulkan
    let vk_entry = EntryLoader::new()?;

    // Check to see if OpenXR and Vulkan are compatible
    let vk_version = info.api_version;
    //unsafe { vk_entry.enumerate_instance_version(None).result()? };

    let xr_vk_version = xr::Version::new(
        vk::version_major(vk_version) as u16,
        vk::version_minor(vk_version) as u16,
        vk::version_patch(vk_version),
    );

    println!("Loaded Vulkan version {}", vk_version);
    let reqs = xr_instance
        .graphics_requirements::<xr::Vulkan>(system)
        .unwrap();
    if reqs.min_api_version_supported > xr_vk_version {
        bail!(
            "OpenXR runtime requires Vulkan version > {}",
            reqs.min_api_version_supported
        );
    }

    // Vulkan Instance
    let application_name = CString::new(info.name)?;
    let engine_name = CString::new(crate::ENGINE_NAME)?;
    let app_info = vk::ApplicationInfoBuilder::new()
        .application_name(&application_name)
        .application_version(vk::make_version(1, 0, 0))
        .engine_name(&engine_name)
        .engine_version(crate::engine_version())
        .api_version(info.api_version);

    // Instance and device layers and extensions
    let mut vk_instance_layers = Vec::new();
    let mut vk_instance_extensions = Vec::new();
    let mut vk_device_layers = Vec::new();
    let vk_device_extensions = Vec::new();

    if info.validation {
        const LAYER_KHRONOS_VALIDATION: *const i8 = cstr!("VK_LAYER_KHRONOS_validation");
        vk_instance_extensions
            .push(erupt::extensions::ext_debug_utils::EXT_DEBUG_UTILS_EXTENSION_NAME);
        vk_instance_layers.push(LAYER_KHRONOS_VALIDATION);
        vk_device_layers.push(LAYER_KHRONOS_VALIDATION);
    }

    // Get Instance from OpenXR
    let create_info = vk::InstanceCreateInfoBuilder::new()
        .application_info(&app_info)
        .enabled_layer_names(&vk_instance_layers)
        .enabled_extension_names(&vk_instance_extensions)
        .build();

    let vk_instance = unsafe { xr_instance.create_vulkan_instance(
        system,
        std::mem::transmute(vk_entry.get_instance_proc_addr),
        &create_info as *const _ as _,
    ) }?.map_err(|_| anyhow::format_err!("OpenXR failed to create Vulkan instance"))?;
    let vk_instance = vk::Instance(vk_instance as _);

    // Create instance loader (for Erupt)
    let symbol = |name| unsafe { (vk_entry.get_instance_proc_addr)(vk_instance, name) };

    let vk_instance_ext_cstrs = unsafe { vk_instance_extensions.iter().map(|&p| CStr::from_ptr(p)).collect::<Vec<_>>() };
    let vk_instance = unsafe {

        let instance_enabled = erupt::InstanceEnabled::new(
            vk_version,
            &vk_instance_ext_cstrs,
            &[],
        )?;
        InstanceLoader::custom(&vk_entry, vk_instance, instance_enabled, symbol)
    }?;

    // Obtain physical vk_device, queue_family_index, and vk_device from OpenXR
    let vk_physical_device = vk::PhysicalDevice(
        xr_instance
        .vulkan_graphics_device(system, vk_instance.handle.0 as _)
        .unwrap() as _,
    );

    let queue_family_index = unsafe {
        vk_instance
            .get_physical_device_queue_family_properties(vk_physical_device, None)
            .into_iter()
            .enumerate()
            .filter_map(|(queue_family_index, info)| {
                if info.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    Some(queue_family_index as u32)
                } else {
                    None
                }
            })
        .next()
            .context("Vulkan vk_device has no graphics queue")?
    };

    let priorities = [1.0];
    let queues = [vk::DeviceQueueCreateInfoBuilder::new()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities)];
    let mut create_info = vk::DeviceCreateInfoBuilder::new()
        .queue_create_infos(&queues)
        .enabled_layer_names(&vk_device_layers)
        .enabled_extension_names(&vk_device_extensions)
        .build();

    let mut phys_device_features = erupt::vk1_2::PhysicalDeviceVulkan11Features {
        multiview: vk::TRUE,
        ..Default::default()
    };

    create_info.p_next = &mut phys_device_features as *mut _ as _;


    todo!()
}
