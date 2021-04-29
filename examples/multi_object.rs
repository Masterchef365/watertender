#![allow(unused)]
use anyhow::Result;
use shortcuts::{
    create_render_pass, launch,
    mesh::*,
    shader,
    starter_kit::{self, StarterKit},
    FrameDataUbo, FramebufferManager, ManagedBuffer, MultiPlatformCamera, StagingBuffer,
    Synchronization, UsageFlags, Vertex,
};

use watertender::*;

const FRAMES_IN_FLIGHT: usize = 2;

struct App {
    rainbow_cube: ManagedMesh,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    scene_ubo: FrameDataUbo<SceneData>,
    camera: MultiPlatformCamera,
    anim: f32,
    starter_kit: StarterKit,
}

fn main() -> Result<()> {
    let info = AppInfo::default().validation(true);
    let vr = std::env::args().count() > 1;
    launch::<App>(info, vr)
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct SceneData {
    cameras: [f32; 4 * 4 * 2],
    anim: f32,
}

unsafe impl bytemuck::Zeroable for SceneData {}
unsafe impl bytemuck::Pod for SceneData {}

impl MainLoop for App {
    fn new(core: &SharedCore, mut platform: Platform<'_>) -> Result<Self> {
        // Frame-frame sync
        let sync = Synchronization::new(
            core.clone(),
            FRAMES_IN_FLIGHT,
            matches!(platform, Platform::Winit { .. }),
        )?;

        // Freambuffer and render pass
        let framebuffer = FramebufferManager::new(core.clone(), platform.is_vr());
        let render_pass = create_render_pass(&core, platform.is_vr())?;

        // Camera
        let camera = MultiPlatformCamera::new(&mut platform);

        const SCENE_DATA_BINDING: u32 = 0;
        let bindings = [vk::DescriptorSetLayoutBindingBuilder::new()
            .binding(SCENE_DATA_BINDING)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT)];

        let descriptor_set_layout_ci =
            vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

        let descriptor_set_layout = unsafe {
            core.device
                .create_descriptor_set_layout(&descriptor_set_layout_ci, None, None)
        }
        .result()?;

        // Scene data
        let scene_ubo = FrameDataUbo::new(
            core.clone(),
            FRAMES_IN_FLIGHT,
            descriptor_set_layout,
            SCENE_DATA_BINDING,
        )?;

        let descriptor_set_layouts = [descriptor_set_layout];

        // Pipeline layout
        let push_constant_ranges = [vk::PushConstantRangeBuilder::new()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<[f32; 4 * 4]>() as u32)];

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

        // Mesh uploads
        let mut staging_buffer = StagingBuffer::new(core.clone())?;
        let (vertices, indices) = rainbow_cube();
        let rainbow_cube =
            upload_mesh(&mut staging_buffer, command_buffers[0], &vertices, &indices)?;

        let starter_kit = StarterKit::new(core.clone(), &mut platform)?;

        let camera = MultiPlatformCamera::new(&mut platform);

        Ok(Self {
            camera,
            anim: 0.0,
            pipeline_layout,
            scene_ubo,
            rainbow_cube,
            pipeline,
            starter_kit,
        })
    }

    fn frame(
        &mut self,
        frame: Frame,
        core: &SharedCore,
        platform: Platform<'_>,
    ) -> Result<PlatformReturn> {
        let cmd = self.starter_kit.begin_command_buffer(frame)?;
        let command_buffer = cmd.command_buffer;

        unsafe {
            core.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &[self.scene_ubo.descriptor_set(self.starter_kit.frame)],
                &[],
            );

            // Draw cmds
            core.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );

            draw_meshes(
                core,
                command_buffer,
                std::slice::from_ref(&self.rainbow_cube),
            );
        }

        let (ret, cameras) = self.camera.get_matrices(platform)?;

        self.scene_ubo.upload(
            self.starter_kit.frame,
            &SceneData {
                cameras,
                anim: self.anim,
            },
        );

        // End draw cmds
        self.starter_kit.end_command_buffer(cmd)?;

        Ok(ret)
    }

    fn swapchain_resize(&mut self, images: Vec<vk::Image>, extent: vk::Extent2D) -> Result<()> {
        self.starter_kit.swapchain_resize(images, extent)
    }

    fn event(
        &mut self,
        mut event: PlatformEvent<'_, '_>,
        core: &Core,
        mut platform: Platform<'_>,
    ) -> Result<()> {
        self.camera.handle_event(&mut event, &mut platform);
        starter_kit::close_when_asked(event, platform);
        Ok(())
    }
}

impl SyncMainLoop for App {
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.starter_kit.winit_sync()
    }
}

/*
// Vertex buffer
let vertices = vec![
Vertex::new([-0.5, 0.5, 0.], [1., 0., 0.]),
Vertex::new([0.5, 0.5, 0.], [0., 1., 0.]),
Vertex::new([0.0, -1.0, 0.], [0., 0., 1.]),
];

let indices = vec![0, 1, 2];
*/
fn rainbow_cube() -> (Vec<Vertex>, Vec<u32>) {
    let vertices = vec![
        Vertex::new([-1.0, -1.0, -1.0], [0.0, 1.0, 1.0]),
        Vertex::new([1.0, -1.0, -1.0], [1.0, 0.0, 1.0]),
        Vertex::new([1.0, 1.0, -1.0], [1.0, 1.0, 0.0]),
        Vertex::new([-1.0, 1.0, -1.0], [0.0, 1.0, 1.0]),
        Vertex::new([-1.0, -1.0, 1.0], [1.0, 0.0, 1.0]),
        Vertex::new([1.0, -1.0, 1.0], [1.0, 1.0, 0.0]),
        Vertex::new([1.0, 1.0, 1.0], [0.0, 1.0, 1.0]),
        Vertex::new([-1.0, 1.0, 1.0], [1.0, 0.0, 1.0]),
    ];

    let indices = vec![
        3, 1, 0, 2, 1, 3, 2, 5, 1, 6, 5, 2, 6, 4, 5, 7, 4, 6, 7, 0, 4, 3, 0, 7, 7, 2, 3, 6, 2, 7,
        0, 5, 4, 1, 5, 0,
    ];

    (vertices, indices)
}
