use crate::hardware_query::HardwareSelection;
use crate::{AppInfo, Core, Platform, PlatformEvent, WinitMainLoop, Frame};
use anyhow::{Context, Result};
use erupt::{
    cstr,
    extensions::{khr_surface, khr_swapchain},
    utils::surface,
    vk1_0 as vk, DeviceLoader, EntryLoader, InstanceLoader,
};
use gpu_alloc::GpuAllocator;
use std::ffi::CString;
use std::sync::Mutex;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

pub fn launch<M: WinitMainLoop + 'static>(info: AppInfo) -> Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(&info.name)
        .build(&event_loop)
        .context("Failed to create window")?;

    let core = build_core(info, &window)?;
    begin_loop::<M>(core, event_loop, window)
}

// TODO: Swap this out for better behaviour! (At least gracefully exit sorta)
fn res<T>(r: Result<T>) -> T {
    match r {
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(-1)
        },
        Ok(v) => v,
    }
}

pub fn begin_loop<M: WinitMainLoop + 'static>(
    core: Core,
    event_loop: EventLoop<()>,
    window: Window,
) -> Result<()> {
    let mut app = M::new(
        &core,
        Platform::Winit {
            window: &window,
            flow: &mut Default::default(),
        },
    )?;

    let mut swapchain = res(Swapchain::new(&core));

    event_loop.run(move |event, _, control_flow| {
        let platform = Platform::Winit {
            window: &window,
            flow: control_flow,
        };
        res(app.event(PlatformEvent::Winit(&event), &core, platform));

        match event {
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let (image_available, render_finished) = app.winit_sync();
                let (swapchain_index, resize) = res(swapchain.frame(image_available));
                let frame = Frame { swapchain_index };
                if let Some((images, extent)) = resize {
                    app.swapchain_resize(images, extent);
                }
                res(app.frame(frame, &core, platform));
                // Submit frame to swapchain
            }
            _ => (),
        }
    });
}

pub fn build_core(info: AppInfo, window: &Window) -> Result<Core> {
    // Entry
    let entry = EntryLoader::new()?;

    // Instance
    let app_name = CString::new(info.name)?;
    let engine_name = CString::new(crate::ENGINE_NAME)?;
    let app_info = vk::ApplicationInfoBuilder::new()
        .application_name(&app_name)
        .application_version(info.version)
        .engine_name(&engine_name)
        .engine_version(crate::engine_version())
        .api_version(info.api_version);

    // Instance and device layers and extensions
    let mut instance_layers = Vec::new();
    let mut instance_extensions = surface::enumerate_required_extensions(window).result()?;
    let mut device_layers = Vec::new();
    let device_extensions = vec![khr_swapchain::KHR_SWAPCHAIN_EXTENSION_NAME];

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

    let mut instance = InstanceLoader::new(&entry, &create_info, None)?;

    // Surface
    let surface = unsafe { surface::create_surface(&mut instance, window, None) }.result()?;

    // Hardware selection
    let hardware = HardwareSelection::query(&instance, surface, &device_extensions)?;

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
        gpu_alloc::Config::i_am_prototyping(),
        device_props,
    ));

    Ok(Core {
        physical_device: hardware.physical_device,
        queue_family: hardware.queue_family,
        queue,
        device,
        instance,
        allocator,
        entry,
    })
}
