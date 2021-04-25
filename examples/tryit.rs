#![allow(unused)]
use anyhow::Result;
use shortcuts::{
    create_render_pass, shader, FramebufferManager, MemObject, Synchronization, UsageFlags, Vertex,
};
use watertender::*;

const FRAMES_IN_FLIGHT: usize = 2;

struct App {
    framebuffer: FramebufferManager,
    sync: Synchronization,
    render_pass: vk::RenderPass,
    vertex_buffer: MemObject<vk::Buffer>,
    frame: usize,
    pipeline: vk::Pipeline,
    command_buffers: Vec<vk::CommandBuffer>,
    //descriptor_sets: Vec<vk::DescriptorSet>,
}

fn main() -> Result<()> {
    if std::env::args().count() > 1 {
        openxr_backend::launch::<App>(Default::default())
    } else {
        winit_backend::launch::<App>(Default::default())
    }
}

impl MainLoop for App {
    fn new(core: &SharedCore, platform: Platform<'_>) -> Result<Self> {
        // External stuff
        let sync = Synchronization::new(
            core.clone(),
            FRAMES_IN_FLIGHT,
            matches!(platform, Platform::Winit { .. }),
        )?;

        let framebuffer = FramebufferManager::new(core.clone(), platform.is_vr());
        let render_pass = create_render_pass(&core, platform.is_vr())?;

        // Vertex buffer
        let vertices = vec![
            Vertex::new([-0.5, 0.5, 0.], [1., 0., 0.]),
            Vertex::new([0.5, 0.5, 0.], [0., 1., 0.]),
            Vertex::new([0.0, -1.0, 0.], [0., 0., 1.]),
        ];

        let create_info = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .size(std::mem::size_of_val(vertices.as_slice()) as u64);

        let mut vertex_buffer = MemObject::new_buffer(core, create_info, UsageFlags::UPLOAD)?;
        vertex_buffer.write_bytes(core, 0, bytemuck::cast_slice(&vertices))?;

        // Create descriptor layout
        let bindings = [
            vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(0)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX),
            vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(1)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT),
        ];

        let descriptor_set_layout_ci =
            vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

        let descriptor_set_layout = unsafe {
            core.device
                .create_descriptor_set_layout(&descriptor_set_layout_ci, None, None)
        }
        .result()?;

        let descriptor_set_layouts = [descriptor_set_layout];

        // Create descriptor pool
        /*
        let pool_sizes = [vk::DescriptorPoolSizeBuilder::new()
            ._type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count((FRAMES_IN_FLIGHT * 2) as u32)];
        let create_info = vk::DescriptorPoolCreateInfoBuilder::new()
            .pool_sizes(&pool_sizes)
            .max_sets(FRAMES_IN_FLIGHT as u32);
        let descriptor_pool = unsafe {
            core
                .device
                .create_descriptor_pool(&create_info, None, None)
        }
        .result()?;

        // Create descriptor sets
        let layouts = vec![descriptor_set_layout; FRAMES_IN_FLIGHT];
        let create_info = vk::DescriptorSetAllocateInfoBuilder::new()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets =
            unsafe { core.device.allocate_descriptor_sets(&create_info) }.result()?;
        */

        // Pipeline layout
        let push_constant_ranges = [vk::PushConstantRangeBuilder::new()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<[f32; 16]>() as u32)];

        let create_info = vk::PipelineLayoutCreateInfoBuilder::new()
            .push_constant_ranges(&push_constant_ranges)
            .set_layouts(&descriptor_set_layouts);

        let pipeline_layout =
            unsafe { core.device.create_pipeline_layout(&create_info, None, None) }.result()?;

        // Pipeline
        let pipeline = shader(
            core,
            include_bytes!("unlit.vert.spv"),
            include_bytes!("unlit.frag.spv"),
            vk::PrimitiveTopology::TRIANGLE_LIST,
            render_pass,
            pipeline_layout,
        )?;

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

        Ok(App {
            sync,
            command_buffers,
            pipeline,
            framebuffer,
            render_pass,
            vertex_buffer,
            frame: 0,
        })
    }

    fn frame(&mut self, frame: Frame, core: &SharedCore, platform: Platform<'_>) -> Result<()> {
        let command_buffer = self.command_buffers[self.frame];
        let framebuffer = self.framebuffer.frame(frame.swapchain_index);

        unsafe {
            core.device
                .reset_command_buffer(command_buffer, None)
                .result()?;

            let begin_info = vk::CommandBufferBeginInfoBuilder::new();
            core.device
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

            core.device.cmd_begin_render_pass(
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

            core.device.cmd_end_render_pass(command_buffer);

            core.device.end_command_buffer(command_buffer).result()?;
        }

        let fence = self.sync.sync(frame.swapchain_index, self.frame)?;
        let command_buffers = [command_buffer];
        let (wait_semaphores, signal_semaphores) = if let Some((image_available, render_finished)) =
            self.sync.swapchain_sync(self.frame)
        {
            (vec![image_available], vec![render_finished])
        } else {
            (vec![], vec![])
        };

        let submit_info = vk::SubmitInfoBuilder::new()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
            .command_buffers(&command_buffers)
            .signal_semaphores(&signal_semaphores);
        unsafe {
            core.device
                .queue_submit(core.queue, &[submit_info], Some(fence))
                .result()?;
        }

        self.frame = (self.frame + 1) % FRAMES_IN_FLIGHT;
        Ok(())
    }

    fn swapchain_resize(&mut self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()> {
        self.framebuffer.resize(images, extent, self.render_pass)
    }

    fn event(
        &mut self,
        event: PlatformEvent<'_, '_>,
        core: &Core,
        platform: Platform<'_>,
    ) -> Result<()> {
        if let PlatformEvent::Winit(winit::event::Event::WindowEvent { event, .. }) = event {
            if let winit::event::WindowEvent::CloseRequested = event {
                if let Platform::Winit {
                    flow, ..
                } = platform {
                    *flow = winit::event_loop::ControlFlow::Exit;
                }
            }
        }
        Ok(())
    }
}

impl WinitMainLoop for App {
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.sync
            .swapchain_sync(self.frame)
            .expect("khr_sync not set")
    }
}

impl Drop for App {
    fn drop(&mut self) {
        // TODO: Make those objects auto-free...
        //self.vertex_buffer.free(&self.core);
    }
}
