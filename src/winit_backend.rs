use crate::hardware_query::HardwareSelection;
use crate::{AppInfo, Core, Frame, Platform, PlatformEvent, SharedCore, SyncMainLoop};
use anyhow::{Context, Result};
use erupt::{
    cstr,
    extensions::{
        khr_surface::{self, PresentModeKHR, SurfaceKHR},
        khr_swapchain::{self, SwapchainKHR},
    },
    utils::surface,
    vk, DeviceLoader, EntryLoader, InstanceLoader,
};
use gpu_alloc::GpuAllocator;
use std::ffi::CString;
use std::sync::Mutex;
use winit::{
    event::Event,
    event_loop::EventLoop,
    window::{Window, WindowBuilder},
};

pub fn launch<M: SyncMainLoop + 'static>(info: AppInfo) -> Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(&info.name)
        .build(&event_loop)
        .context("Failed to create window")?;

    let (core, surface, present_mode) = build_core(info, &window)?;
    begin_loop::<M>(core, event_loop, window, surface, present_mode)
}

// TODO: Swap this out for better behaviour! (At least sorta exit gracefully...)
fn res<T>(r: Result<T>) -> T {
    match r {
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(-1)
        }
        Ok(v) => v,
    }
}

fn begin_loop<M: SyncMainLoop + 'static>(
    core: Core,
    event_loop: EventLoop<()>,
    window: Window,
    surface: SurfaceKHR,
    present_mode: PresentModeKHR,
) -> Result<()> {
    let core = SharedCore::new(core);

    let mut app = M::new(
        &core,
        Platform::Winit {
            window: &window,
            control_flow: &mut Default::default(),
        },
    )?;

    let (mut swapchain, (images, extent)) =
        res(Swapchain::new(core.clone(), surface, present_mode));
    res(app.swapchain_resize(images, extent));

    event_loop.run(move |event, _, control_flow| {
        res(app.event(
            PlatformEvent::Winit(&event),
            &core,
            Platform::Winit {
                window: &window,
                control_flow,
            },
        ));

        match event {
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let (image_available, render_finished) = app.winit_sync();
                let (swapchain_index, resize) = res(swapchain.frame(image_available));
                let frame = Frame { swapchain_index };
                if let Some((images, extent)) = resize {
                    res(app.swapchain_resize(images, extent));
                }
                res(app.frame(
                    frame,
                    &core,
                    Platform::Winit {
                        window: &window,
                        control_flow,
                    },
                ));
                res(swapchain.queue_present(swapchain_index, render_finished));
            }
            _ => (),
        }
    });
}

pub fn build_core(info: AppInfo, window: &Window) -> Result<(Core, SurfaceKHR, PresentModeKHR)> {
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
        gpu_alloc::Config::i_am_prototyping(), // TODO: SET THIS TO SOMETHING MORE SANE!! Maybe embed in AppInfo?!
        device_props,
    ));
    let device_properties = unsafe { instance.get_physical_device_properties(hardware.physical_device, None) };

    let core = Core {
        physical_device: hardware.physical_device,
        device_properties,
        queue_family: hardware.queue_family,
        queue,
        device,
        instance,
        allocator,
        entry,
    };

    Ok((core, surface, hardware.present_mode))
}

struct Swapchain {
    inner: SwapchainKHR,
    surface: SurfaceKHR,
    core: SharedCore,
    present_mode: PresentModeKHR,
}

type SwapchainImages = (Vec<vk::Image>, vk::Extent2D);

impl Swapchain {
    pub fn new(
        core: SharedCore,
        surface: SurfaceKHR,
        present_mode: PresentModeKHR,
    ) -> Result<(Self, SwapchainImages)> {
        let (inner, images) = Self::create_swapchain(&core, surface, present_mode, None)?;
        let instance = Self {
            inner,
            surface,
            core,
            present_mode,
        };
        Ok((instance, images))
    }

    pub fn frame(
        &mut self,
        image_available: vk::Semaphore,
    ) -> Result<(u32, Option<SwapchainImages>)> {
        let ret = self.acquire_image(image_available);

        // Early return and invalidate swapchain
        if ret.raw == vk::Result::ERROR_OUT_OF_DATE_KHR {
            let resize = self.rebuild_swapchain()?;
            let img_idx = self.acquire_image(image_available).result()?; // Fail if we already tried once
            Ok((img_idx, Some(resize)))
        } else {
            Ok((ret.result()?, None))
        }
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

    fn free_swapchain(&mut self) {
        unsafe {
            self.core
                .device
                .destroy_swapchain_khr(Some(self.inner), None);
        }
    }

    fn create_swapchain(
        core: &Core,
        surface: SurfaceKHR,
        present_mode: PresentModeKHR,
        old_swapchain: Option<SwapchainKHR>,
    ) -> Result<(SwapchainKHR, SwapchainImages)> {
        let surface_caps = unsafe {
            core.instance.get_physical_device_surface_capabilities_khr(
                core.physical_device,
                surface,
                None,
            )
        }
        .result()?;

        let mut image_count = surface_caps.min_image_count + 1;
        if surface_caps.max_image_count > 0 && image_count > surface_caps.max_image_count {
            image_count = surface_caps.max_image_count;
        }

        // Build the actual swapchain
        let create_info = khr_swapchain::SwapchainCreateInfoKHRBuilder::new()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(crate::COLOR_FORMAT)
            .image_color_space(crate::COLOR_SPACE)
            .image_extent(surface_caps.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_caps.current_transform)
            .composite_alpha(khr_surface::CompositeAlphaFlagBitsKHR::OPAQUE_KHR)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(match old_swapchain {
                Some(s) => s,
                None => SwapchainKHR::null()
            });

        let swapchain =
            unsafe { core.device.create_swapchain_khr(&create_info, None, None) }.result()?;

        let swapchain_images =
            unsafe { core.device.get_swapchain_images_khr(swapchain, None) }.result()?;

        Ok((swapchain, (swapchain_images, surface_caps.current_extent)))
    }

    fn queue_present(
        &mut self,
        image_index: u32,
        render_finished: vk::Semaphore,
    ) -> Result<()> {
        // Present to swapchain
        let swapchains = [self.inner];
        let image_indices = [image_index];
        let wait_semaphores = [render_finished];
        let present_info = khr_swapchain::PresentInfoKHRBuilder::new()
            .wait_semaphores(&wait_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        // TODO: Handle queue result?
        let _ = unsafe {
            self.core
                .device
                .queue_present_khr(self.core.queue, &present_info)
        };

        Ok(())
    }

    fn rebuild_swapchain(&mut self) -> Result<SwapchainImages> {
        let (swapchain, resize) =
            Self::create_swapchain(&self.core, self.surface, self.present_mode, Some(self.inner))?;
        self.free_swapchain();
        self.inner = swapchain;
        Ok(resize)
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        self.free_swapchain();
        unsafe {
            self.core
                .instance
                .destroy_surface_khr(Some(self.surface), None);
        }
    }
}
