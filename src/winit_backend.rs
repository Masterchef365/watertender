use crate::{MainLoop, AppInfo, Core, SharedCore};

pub fn launch<M: MainLoop>(info: AppInfo) -> Result<()> {
    todo!()
}

use crate::hardware_query::HardwareSelection;
use crate::swapchain_images::SwapchainImages;
use anyhow::Result;
use erupt::{
    extensions::{khr_surface, khr_swapchain},
    utils::surface,
    vk1_0 as vk, DeviceLoader, EntryLoader, InstanceLoader,
};
use std::ffi::CString;
use winit::window::Window;
use std::sync::Mutex;
use gpu_alloc::GpuAllocator;

/// Winit engine backend
pub struct WinitBackend {
    swapchain: Option<Swapchain>,
    image_available_semaphores: Vec<vk::Semaphore>,
    surface: khr_surface::SurfaceKHR,
    hardware: HardwareSelection,
    frame_idx: usize,
    core: SharedCore,
}

/// Content recreated on swapchain invalidation
struct Swapchain {
    handle: khr_swapchain::SwapchainKHR,
    images: SwapchainImages,
}

impl WinitBackend {
    /// Create a new engine instance.
    pub fn new(window: &Window, application_name: &str) -> Result<Self> {
        // Entry
        let entry = EntryLoader::new()?;

        // Instance
        let application_name = CString::new(application_name)?;
        let engine_name = CString::new(crate::ENGINE_NAME)?;
        let app_info = vk::ApplicationInfoBuilder::new()
            .application_name(&application_name)
            .application_version(vk::make_version(1, 0, 0))
            .engine_name(&engine_name)
            .engine_version(crate::engine_version())
            .api_version(vk::make_version(1, 0, 0));

        // Instance and device layers and extensions
        let mut instance_layers = Vec::new();
        let mut instance_extensions = surface::enumerate_required_extensions(window).result()?;
        let mut device_layers = Vec::new();
        let mut device_extensions = vec![khr_swapchain::KHR_SWAPCHAIN_EXTENSION_NAME];

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
        let graphics_queue = unsafe { device.get_device_queue(hardware.queue_family, 0, None) };
        let utility_queue = unsafe { device.get_device_queue(hardware.queue_family, 1, None) };

        let device_props = unsafe { gpu_alloc_erupt::device_properties(&instance, hardware.physical_device)? };

        // TODO: Switch away from prototype mode! (Perhaps an option in AppInfo?
        let allocator =
            Mutex::new(GpuAllocator::new(gpu_alloc::Config::i_am_prototyping(), device_props));


        let core = SharedCore::new(crate::Core {
            graphics_queue,
            utility_queue,
            device,
            instance,
            allocator,
            entry,
        });


        let image_available_semaphores = (0..crate::FRAMES_IN_FLIGHT)
            .map(|_| {
                let create_info = vk::SemaphoreCreateInfoBuilder::new();
                unsafe {
                    core
                        .device
                        .create_semaphore(&create_info, None, None)
                        .result()
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            swapchain: None,
            image_available_semaphores,
            hardware,
            surface,
            frame_idx: 0,
            core,
        })
    }

    pub fn next_frame<App: MainLoop>(&mut self, app: &mut App, core: &Core) -> Result<()> {
        if self.swapchain.is_none() {
            self.swapchain = Some(self.create_swapchain(app.renderpass())?);
        }
        let swapchain = self.swapchain.as_mut().unwrap(); // Unreachable unwrap!

        let image_available = self.image_available_semaphores[self.frame_idx];
        let image_index = unsafe {
            self.core.device.acquire_next_image_khr(
                swapchain.handle,
                u64::MAX,
                Some(image_available),
                None,
                None,
            )
        };

        // Early return and invalidate swapchain
        let image_index = if image_index.raw == vk::Result::ERROR_OUT_OF_DATE_KHR {
            self.free_swapchain()?;
            return Ok(());
        } else {
            image_index.unwrap()
        };

        //let image: crate::swapchain_images::SwapChainImage = todo!();
        let image = {
            self
                .swapchain
                .as_mut()
                .expect("Swapchain never assigned!")
                .images
                .next_image(image_index, &in_flight_fence)?
        };

        // Write command buffers
        let command_buffer = self.core.write_command_buffers(frame_idx, packet, &image)?;

        // Submit to the queue
        let command_buffers = [command_buffer];
        let wait_semaphores = [image_available];
        let signal_semaphores = [frame.render_finished];
        let submit_info = vk::SubmitInfoBuilder::new()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
        unsafe {
            self.core
                .device
                .reset_fences(&[frame.in_flight_fence])
                .result()?; // TODO: Move this into the swapchain next_image
            self.core
                .device
                .queue_submit(
                    self.core.queue,
                    &[submit_info],
                    Some(frame.in_flight_fence),
                )
                .result()?;
        }

        // Present to swapchain
        let swapchains = [swapchain];
        let image_indices = [image_index];
        let present_info = khr_swapchain::PresentInfoKHRBuilder::new()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        let queue_result = unsafe {
            self.core
                .device
                .queue_present_khr(self.core.queue, &present_info)
        };

        if queue_result.raw == vk::Result::ERROR_OUT_OF_DATE_KHR {
            self.free_swapchain()?;
            return Ok(());
        } else {
            queue_result.result()?;
        };

        Ok(())
    }

    fn free_swapchain(&mut self) -> Result<()> {
        if let Some(swapchain) = self.swapchain.take() {
            drop(swapchain.images);
            unsafe {
                self.core
                    .device
                    .destroy_swapchain_khr(Some(swapchain.handle), None);
            }
        }
        Ok(())
    }

    fn create_swapchain(&mut self, render_pass: vk::RenderPass) -> Result<Swapchain> {
        let surface_caps = unsafe {
            self.core
                .instance
                .get_physical_device_surface_capabilities_khr(
                    self.hardware.physical_device,
                    self.surface,
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
            .surface(self.surface)
            .min_image_count(image_count)
            .image_format(crate::COLOR_FORMAT)
            .image_color_space(self.hardware.format.color_space)
            .image_extent(surface_caps.current_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_caps.current_transform)
            .composite_alpha(khr_surface::CompositeAlphaFlagBitsKHR::OPAQUE_KHR)
            .present_mode(self.hardware.present_mode)
            .clipped(true)
            .old_swapchain(khr_swapchain::SwapchainKHR::null());

        let handle = unsafe {
            self.core
                .device
                .create_swapchain_khr(&create_info, None, None)
        }
        .result()?;
        let images = unsafe {
            self.core
                .device
                .get_swapchain_images_khr(handle, None)
        }
        .result()?;

        // TODO: Coagulate these two into one object?

        let images = Some(SwapchainImages::new(
            self.core.clone(),
            surface_caps.current_extent,
            render_pass,
            images,
            false,
        )?);

        Ok(Swapchain {
            images,
            handle,
        })
    }
}

impl Drop for WinitBackend {
    fn drop(&mut self) {
        unsafe {
            for semaphore in self.image_available_semaphores.drain(..) {
                self.core.device.destroy_semaphore(Some(semaphore), None);
            }
            self.free_swapchain().unwrap();
            self.core
                .instance
                .destroy_surface_khr(Some(self.surface), None);
        }
    }
}
