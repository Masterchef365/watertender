use crate::{AppInfo, Core, MainLoop, Platform, PlatformEvent, PlatformReturn, SharedCore};
use anyhow::{bail, ensure, Context, Result};
use erupt::{cstr, vk, DeviceLoader, EntryLoader, InstanceLoader};
use gpu_alloc::{self, GpuAllocator};
use openxr as xr;
use std::ffi::{CStr, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type SharedXrCore = Arc<XrCore>;

/// A container for several commonly-used OpenXR constants.
pub struct XrCore {
    pub instance: xr::Instance,
    pub session: xr::Session<xr::Vulkan>,
    pub system: xr::SystemId,
    pub stage: xr::Space,
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

    let (core, xr_core, frame_stream, mut frame_waiter) = build_cores(info)?;
    let mut swapchain = Swapchain::new(xr_core.clone(), frame_stream)?;
    let mut app = M::new(
        &core,
        Platform::OpenXr {
            xr_core: &xr_core,
            frame_state: None,
        },
    )?;

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
                Platform::OpenXr {
                    xr_core: &xr_core,
                    frame_state: None,
                },
            )?;
        }

        if !session_running {
            // Don't grind up the CPU
            std::thread::sleep(Duration::from_millis(100));
            continue;
        }

        // Get next frame
        let xr_frame_state = frame_waiter.wait()?; // TODO: Move this around for better latency?

        let (swapchain_index, resize) = swapchain.frame(xr_frame_state)?;
        let swapchain_index = match swapchain_index {
            Some(i) => i,
            None => continue, // Don't draw
        };

        // Resize swapchain if necessary
        if let Some((images, extent)) = resize {
            app.swapchain_resize(images, extent)?;
        }

        // Run the app
        let ret = app.frame(
            crate::Frame { swapchain_index },
            &core,
            Platform::OpenXr {
                xr_core: &xr_core,
                frame_state: Some(xr_frame_state),
            },
        )?;
        let views = match ret {
            PlatformReturn::OpenXr(v) => v,
            _ => bail!("Wrong platform return"),
        };

        // Present the image
        swapchain.queue_present(xr_frame_state, views)?;
    }
}

fn build_cores(
    info: AppInfo,
) -> Result<(
    SharedCore,
    SharedXrCore,
    xr::FrameStream<xr::Vulkan>,
    xr::FrameWaiter,
)> {
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

    println!("Loaded Vulkan version {}", xr_vk_version);
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
        .application_version(info.version)
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

    let vk_instance = unsafe {
        xr_instance.create_vulkan_instance(
            system,
            std::mem::transmute(vk_entry.get_instance_proc_addr),
            &create_info as *const _ as _,
        )
    }?
    .map_err(|_| anyhow::format_err!("OpenXR failed to create Vulkan instance"))?;
    let vk_instance = vk::Instance(vk_instance as _);

    // Create instance loader (for Erupt)
    let symbol = |name| unsafe { (vk_entry.get_instance_proc_addr)(vk_instance, name) };

    let vk_instance_ext_cstrs = unsafe {
        vk_instance_extensions
            .iter()
            .map(|&p| CStr::from_ptr(p))
            .collect::<Vec<_>>()
    };
    let vk_instance = unsafe {
        let instance_enabled =
            erupt::InstanceEnabled::new(vk_version, &vk_instance_ext_cstrs, &[])?;
        InstanceLoader::custom(&vk_entry, vk_instance, instance_enabled, symbol)
    }?;

    // Obtain physical vk_device
    let vk_physical_device = vk::PhysicalDevice(
        xr_instance
            .vulkan_graphics_device(system, vk_instance.handle.0 as _)
            .unwrap() as _,
    );

    // Get queue
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

    // Create device
    let priorities = [1.0];
    let queues = [vk::DeviceQueueCreateInfoBuilder::new()
        .queue_family_index(queue_family_index)
        .queue_priorities(&priorities)];

    let mut create_info = vk::DeviceCreateInfoBuilder::new()
        .queue_create_infos(&queues)
        .enabled_layer_names(&vk_device_layers)
        .enabled_extension_names(&vk_device_extensions)
        .build();

    // Enable multiview
    let mut phys_device_features = erupt::vk1_2::PhysicalDeviceVulkan11Features {
        multiview: vk::TRUE,
        ..Default::default()
    };

    create_info.p_next = &mut phys_device_features as *mut _ as _;

    // Get Vulkan Device from OpenXR
    let vk_device = unsafe {
        xr_instance.create_vulkan_device(
            system,
            std::mem::transmute(vk_entry.get_instance_proc_addr),
            vk_physical_device.0 as _,
            &create_info as *const _ as _,
        )
    }?
    .map_err(vk::Result)?;
    let vk_device = vk::Device(vk_device as _);

    // Create DeviceLoader for erupt
    let vk_device_ext_cstrs = unsafe {
        vk_device_extensions
            .iter()
            .map(|&p| CStr::from_ptr(p))
            .collect::<Vec<_>>()
    };
    let device_enabled = unsafe { erupt::DeviceEnabled::new(&vk_device_ext_cstrs) };
    let vk_device =
        unsafe { DeviceLoader::custom(&vk_instance, vk_device, device_enabled, symbol)? };

    // Create queue
    let queue = unsafe { vk_device.get_device_queue(queue_family_index, 0, None) };

    // Create allocator
    let device_props =
        unsafe { gpu_alloc_erupt::device_properties(&vk_instance, vk_physical_device)? };
    let allocator = Mutex::new(GpuAllocator::new(
        gpu_alloc::Config::i_am_prototyping(),
        device_props,
    ));

    // OpenXR session
    let (session, frame_wait, frame_stream) = unsafe {
        xr_instance.create_session::<xr::Vulkan>(
            system,
            &xr::vulkan::SessionCreateInfo {
                instance: vk_instance.handle.0 as _,
                physical_device: vk_physical_device.0 as _,
                device: vk_device.handle.0 as _,
                queue_family_index,
                queue_index: 0,
            },
        )
    }?;

    // Create stage
    let stage = session
        .create_reference_space(xr::ReferenceSpaceType::STAGE, xr::Posef::IDENTITY)
        .unwrap();

    // Create Core
    let core = SharedCore::new(Core {
        queue,
        queue_family: queue_family_index,
        allocator,
        device: vk_device,
        physical_device: vk_physical_device,
        instance: vk_instance,
        entry: vk_entry,
    });

    // Create XrCore
    let xr_core = SharedXrCore::new(XrCore {
        instance: xr_instance,
        session,
        system,
        stage,
    });

    Ok((core, xr_core, frame_stream, frame_wait))
}

pub struct Swapchain {
    frame_stream: xr::FrameStream<xr::Vulkan>,
    swapchain: Option<xr::Swapchain<xr::Vulkan>>,
    xr_core: SharedXrCore,
    current_extent: vk::Extent2D,
}

type SwapchainImages = (Vec<vk::Image>, vk::Extent2D);

impl Swapchain {
    /// Create a new engine instance. Returns the OpenXr caddy for use with input handling.
    pub fn new(
        xr_core: SharedXrCore,
        frame_stream: xr::FrameStream<xr::Vulkan>,
    ) -> Result<Self> {
        Ok(Self {
            swapchain: None,
            frame_stream,
            current_extent: vk::Extent2D::default(),
            xr_core,
        })
    }

    pub fn frame(
        &mut self,
        xr_frame_state: xr::FrameState,
    ) -> Result<(Option<u32>, Option<SwapchainImages>)> {
        // Wait for OpenXR to signal it has a frame ready
        self.frame_stream.begin()?;

        if !xr_frame_state.should_render {
            self.frame_stream.end(
                xr_frame_state.predicted_display_time,
                xr::EnvironmentBlendMode::OPAQUE,
                &[],
            )?;
            return Ok((None, None));
        }

        let resize = if self.swapchain.is_none() {
            Some(self.recreate_swapchain()?)
        } else {
            None
        };

        let swapchain = self.swapchain.as_mut().unwrap();

        let image_index = swapchain.acquire_image()?;

        swapchain.wait_image(xr::Duration::INFINITE)?; // TODO: This should probably go RIGHT BEFORE the submit!

        Ok((Some(image_index), resize))
    }

    pub fn queue_present(
        &mut self,
        xr_frame_state: xr::FrameState,
        views: Vec<xr::View>,
    ) -> Result<()> {
        let swapchain = self.swapchain.as_mut().unwrap();

        // Present to swapchain
        swapchain.release_image()?;

        // Tell OpenXR what to present for this frame
        let rect = xr::Rect2Di {
            offset: xr::Offset2Di { x: 0, y: 0 },
            extent: xr::Extent2Di {
                width: self.current_extent.width as _,
                height: self.current_extent.height as _,
            },
        };
        self.frame_stream.end(
            xr_frame_state.predicted_display_time,
            xr::EnvironmentBlendMode::OPAQUE,
            &[&xr::CompositionLayerProjection::new()
                .space(&self.xr_core.stage)
                .views(&[
                    xr::CompositionLayerProjectionView::new()
                        .pose(views[0].pose)
                        .fov(views[0].fov)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&swapchain)
                                .image_array_index(0)
                                .image_rect(rect),
                        ),
                    xr::CompositionLayerProjectionView::new()
                        .pose(views[1].pose)
                        .fov(views[1].fov)
                        .sub_image(
                            xr::SwapchainSubImage::new()
                                .swapchain(&swapchain)
                                .image_array_index(1)
                                .image_rect(rect),
                        ),
                ])],
        )?;

        Ok(())
    }

    fn recreate_swapchain(&mut self) -> Result<SwapchainImages> {
        self.swapchain = None;

        let views = self
            .xr_core
            .instance
            .enumerate_view_configuration_views(
                self.xr_core.system,
                xr::ViewConfigurationType::PRIMARY_STEREO,
            )
            .unwrap();

        let extent = vk::Extent2D {
            width: views[0].recommended_image_rect_width,
            height: views[0].recommended_image_rect_height,
        };

        let swapchain = self
            .xr_core
            .session
            .create_swapchain(&xr::SwapchainCreateInfo {
                create_flags: xr::SwapchainCreateFlags::EMPTY,
                usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT
                    | xr::SwapchainUsageFlags::SAMPLED,
                format: crate::COLOR_FORMAT.0 as _,
                sample_count: 1,
                width: extent.width,
                height: extent.height,
                face_count: 1,
                array_size: 2,
                mip_count: 1,
            })
            .unwrap();

        let swapchain_images = swapchain
            .enumerate_images()?
            .into_iter()
            .map(vk::Image)
            .collect::<Vec<_>>();

        self.swapchain = Some(swapchain);
        self.current_extent = extent;

        Ok((swapchain_images, extent))
    }
}
