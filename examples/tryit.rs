#![allow(unused)]
use anyhow::Result;
use shortcuts::{
    create_render_pass, FramebufferManager, MemObject, Synchronization, UsageFlags, Vertex,
    shader,
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
    command_buffer: vk::CommandBuffer,
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
            core
                .device
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

        let pipeline_layout = unsafe {
            core
                .device
                .create_pipeline_layout(&create_info, None, None)
        }
        .result()?;

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

        let command_buffer =
            unsafe { core.device.allocate_command_buffers(&allocate_info) }.result()?[0];

        Ok(App {
            sync,
            command_buffer,
            pipeline,
            framebuffer,
            render_pass,
            vertex_buffer,
            frame: 0,
        })
    }

    fn frame(&mut self, frame: Frame, core: &SharedCore, platform: Platform<'_>) -> Result<()> {
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
        if let PlatformEvent::Winit(ev) = event {
            dbg!(ev);
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
