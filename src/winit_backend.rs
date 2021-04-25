use crate::hardware_query::HardwareSelection;
use crate::{AppInfo, Core, Frame, Platform, PlatformEvent, WinitMainLoop, SharedCore};
use anyhow::{Context, Result};
use erupt::{
    cstr,
    extensions::{khr_surface::SurfaceKHR, khr_swapchain::{self, SwapchainKHR}},
    utils::surface,
    vk1_0 as vk, DeviceLoader, EntryLoader, InstanceLoader,
};
use gpu_alloc::GpuAllocator;
use std::ffi::CString;
use std::sync::Mutex;
use winit::{
    event::{Event},
    event_loop::EventLoop,
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
        }
        Ok(v) => v,
    }
}

pub fn begin_loop<M: WinitMainLoop + 'static>(
    core: Core,
    event_loop: EventLoop<()>,
    window: Window,
) -> Result<()> {
    let core = SharedCore::new(core);

    let mut app = M::new(
        &core,
        Platform::Winit {
            window: &window,
            flow: &mut Default::default(),
        },
    )?;

    let mut swapchain = res(Swapchain::new(core.clone()));

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

pub fn build_core(info: AppInfo, window: &Window) -> Result<(Core, SurfaceKHR)> {
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

    let core = Core {
        physical_device: hardware.physical_device,
        queue_family: hardware.queue_family,
        queue,
        device,
        instance,
        allocator,
        entry,
    };

    Ok((core, surface))
}

struct Swapchain {
    inner: SwapchainKHR,
    surface: SurfaceKHR,
    core: SharedCore,
}

impl Swapchain {
    pub fn new(core: SharedCore, surface: SurfaceKHR) -> Result<Self> {
        Ok(Self {
            inner: Self::create_swapchain(&core, surface)?,
            surface,
            core,
        })
    }

    pub fn frame(
        &mut self,
        image_available: vk::Semaphore,
    ) -> Result<(usize, Option<(Vec<vk::Image>, vk::Extent2D)>)> {
        let ret = self.acquire_image(image_available);

        // Early return and invalidate swapchain
        if ret.raw == vk::Result::ERROR_OUT_OF_DATE_KHR {
            self.free_swapchain()?;

            let (swapchain, images, extent) = Self::create_swapchain(&self.core, self.surface)?;

            self.inner = swapchain;

            let resize = (images, extent);
            let img_idx = self.acquire_image(image_available).result()?; // Fail if we already tried once

            Ok((img_idx, Some(resize)))
        } else {
            Ok((ret.result()?, None))
        };
    }

    fn acquire_image(&mut self, image_available: vk::Semaphore) -> erupt::utils::VulkanResult<u32> {
        unsafe {
            self.core.device.acquire_next_image_khr(
                self.inner,
                u64::MAX,
                Some(image_available),
                None,
                None,
            )
        }
    }

    fn free_swapchain(&mut self) -> Result<()> {
    }

    fn create_swapchain(&core, surface: SurfaceKHR) -> Result<(SwapchainKHR, Vec<vk::Image>, vk::Extent2D)> {
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        self.free_swapchain().expect("Failed to free swapchain");
    }
}
