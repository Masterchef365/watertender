use crate::app_info::AppInfo;
use crate::mainloop::{Frame, Platform, PlatformEvent, SyncMainLoop};
use crate::{render_pass::create_render_pass, framebuffer_mgr::FramebufferManager, staging_buffer::StagingBuffer, synchronization::Synchronization};
use crate::SharedCore;
use anyhow::Result;
use erupt::vk;
use crate::defaults::FRAMES_IN_FLIGHT;

/// The StarterKit is a collection of commonly used utilities and code, and is made out of other shortcuts.
pub struct StarterKit {
    pub framebuffer: FramebufferManager,
    pub sync: Synchronization,
    pub render_pass: vk::RenderPass,
    pub staging_buffer: StagingBuffer,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub core: SharedCore,
    pub frame: usize,
}

/// Launch a mainloop, and change platform depending on a boolean
#[cfg(all(feature = "winit", feature = "openxr"))]
pub fn launch<M: SyncMainLoop + 'static>(info: AppInfo, vr: bool) -> anyhow::Result<()> {
    if vr {
        crate::openxr_backend::launch::<M>(info)
    } else {
        crate::winit_backend::launch::<M>(info)
    }
}

/// Run the main loop with validation, and if any command
/// line args are specified, then run in VR mode
pub fn debug<App: SyncMainLoop + 'static>() -> Result<()> {
    let info = AppInfo::default().validation(true);
    let vr = std::env::args().count() > 1;
    launch::<App>(info, vr)
}

/// Constructed by the starter kit; typically just used for it's command buffer and to pass to the
/// `end_command_buffer()` function.
pub struct CommandBufferStart {
    pub command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
}

impl StarterKit {
    pub fn new(core: SharedCore, platform: &mut Platform<'_>) -> Result<Self> {
        // Frame-frame sync
        let sync = Synchronization::new(
            core.clone(),
            FRAMES_IN_FLIGHT,
            matches!(platform, Platform::Winit { .. }),
        )?;

        // Freambuffer and render pass
        let framebuffer = FramebufferManager::new(core.clone(), platform.is_vr());
        let render_pass = create_render_pass(&core, platform.is_vr())?;

        // Command pool
        let create_info = vk::CommandPoolCreateInfoBuilder::new()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(core.queue_family);
        let command_pool =
            unsafe { core.device.create_command_pool(&create_info, None, None) }.result()?;

        // Allocate command buffers
        let allocate_info = vk::CommandBufferAllocateInfoBuilder::new()
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(FRAMES_IN_FLIGHT as u32);

        let command_buffers =
            unsafe { core.device.allocate_command_buffers(&allocate_info) }.result()?;

        // Mesh uploads
        let staging_buffer = StagingBuffer::new(core.clone())?;

        Ok(Self {
            staging_buffer,
            sync,
            command_buffers,
            framebuffer,
            render_pass,
            frame: 0,
            core,
        })
    }

    /// Begins command buffer, render pass, and sets viewports
    pub fn begin_command_buffer(&mut self, frame: Frame) -> Result<CommandBufferStart> {
        let fence = self.sync.sync(frame.swapchain_index, self.frame)?;

        let command_buffer = self.command_buffers[self.frame];
        let framebuffer = self.framebuffer.frame(frame.swapchain_index);

        unsafe {
            self.core
                .device
                .reset_command_buffer(command_buffer, None)
                .result()?;

            let begin_info = vk::CommandBufferBeginInfoBuilder::new();
            self.core
                .device
                .begin_command_buffer(command_buffer, &begin_info)
                .result()?;

            // Set render pass
            let clear_values = [
                vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 0.0, 1.0],
                    },
                },
                vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue {
                        depth: 1.0,
                        stencil: 0,
                    },
                },
            ];

            let begin_info = vk::RenderPassBeginInfoBuilder::new()
                .framebuffer(framebuffer)
                .render_pass(self.render_pass)
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.framebuffer.extent(),
                })
                .clear_values(&clear_values);

            self.core.device.cmd_begin_render_pass(
                command_buffer,
                &begin_info,
                vk::SubpassContents::INLINE,
            );

            let viewports = [vk::ViewportBuilder::new()
                .x(0.0)
                .y(0.0)
                .width(self.framebuffer.extent().width as f32)
                .height(self.framebuffer.extent().height as f32)
                .min_depth(0.0)
                .max_depth(1.0)];

            let scissors = [vk::Rect2DBuilder::new()
                .offset(vk::Offset2D { x: 0, y: 0 })
                .extent(self.framebuffer.extent())];

            self.core
                .device
                .cmd_set_viewport(command_buffer, 0, &viewports);

            self.core
                .device
                .cmd_set_scissor(command_buffer, 0, &scissors);
        }

        Ok(CommandBufferStart {
            command_buffer,
            fence,
        })
    }

    /// End and submit command buffer, and advance to the next frame.
    pub fn end_command_buffer(&mut self, cmd: CommandBufferStart) -> Result<()> {
        let command_buffer = cmd.command_buffer;
        unsafe {
            self.core.device.cmd_end_render_pass(command_buffer);
            self.core
                .device
                .end_command_buffer(command_buffer)
                .result()?;
        }

        let command_buffers = [command_buffer];
        if let Some((image_available, render_finished)) = self.sync.swapchain_sync(self.frame) {
            let wait_semaphores = [image_available];
            let signal_semaphores = [render_finished];
            let submit_info = vk::SubmitInfoBuilder::new()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);
            unsafe {
                self.core
                    .device
                    .queue_submit(self.core.queue, &[submit_info], Some(cmd.fence))
                    .result()?;
            }
        } else {
            let submit_info = vk::SubmitInfoBuilder::new().command_buffers(&command_buffers);
            unsafe {
                self.core
                    .device
                    .queue_submit(self.core.queue, &[submit_info], Some(cmd.fence))
                    .result()?;
            }
        };

        self.frame = (self.frame + 1) % FRAMES_IN_FLIGHT;

        Ok(())
    }

    pub fn current_command_buffer(&self) -> vk::CommandBuffer {
        self.command_buffers[self.frame]
    }

    pub fn swapchain_resize(&mut self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()> {
        self.framebuffer.resize(images, extent, self.render_pass)
    }

    pub fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.sync
            .swapchain_sync(self.frame)
            .expect("khr_sync not set")
    }
}

pub fn close_when_asked(event: PlatformEvent<'_, '_>, platform: Platform<'_>) {
    if let PlatformEvent::Winit(winit::event::Event::WindowEvent { event, .. }) = event {
        if let winit::event::WindowEvent::CloseRequested = event {
            if let Platform::Winit { control_flow, .. } = platform {
                *control_flow = winit::event_loop::ControlFlow::Exit;
            }
        }
    }
}
