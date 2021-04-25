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

        // Descriptor sets
        let descriptor_set_layouts = [descriptor_set_layout];

        // Pipeline layout
        let push_constant_ranges = [vk::PushConstantRangeBuilder::new()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<[f32; 16]>() as u32)];

        let create_info = vk::PipelineLayoutCreateInfoBuilder::new()
            .push_constant_ranges(&push_constant_ranges)
            .set_layouts(&descriptor_set_layouts);

        let pipeline_layout = unsafe {
            prelude
                .device
                .create_pipeline_layout(&create_info, None, None)
        }
        .result()?;

        // Pipeline
        let pipeline = shader(
            core, 
            include_bytes!("examples/unlit.vert.spv"),
            include_bytes!("examples/unlit.frag.spv"),
            vk::PrimitiveTopology::TRIANGLE_LIST,
            render_pass,
            pipeline_layout,
        )?;

        Ok(App {
            sync,
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
        self.vertex_buffer.free(&self.core);
    }
}
