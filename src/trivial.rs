use crate::prelude::*;
use crate::defaults::FRAMES_IN_FLIGHT;
use crate::starter_kit::Settings;
use anyhow::Result;

pub fn draw(draw: DrawList, vr: bool) -> Result<()> {
    let info = AppInfo::default().validation(cfg!(debug_assertions));
    launch::<App, DrawList>(info, vr, draw)
}

/// A list of meshes to draw
pub type DrawList = Vec<DrawData>;

/// A mesh and the primitive it is constructed of
#[derive(Clone)]
pub struct DrawData {
    pub indices: Vec<u32>,
    pub vertices: Vec<Vertex>,
    pub primitive: Primitive,
}

struct App {
    draw: Vec<(ManagedMesh, Primitive)>,

    point_pipeline: vk::Pipeline,
    line_pipeline: vk::Pipeline,
    tri_pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,

    descriptor_sets: Vec<vk::DescriptorSet>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,

    scene_ubo: FrameDataUbo<SceneData>,
    camera: MultiPlatformCamera,
    anim: f32,
    starter_kit: StarterKit,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Primitive {
    Points,
    Lines,
    Triangles,
}

impl Into<vk::PrimitiveTopology> for Primitive {
    fn into(self) -> vk::PrimitiveTopology {
        match self {
            Primitive::Points => vk::PrimitiveTopology::POINT_LIST,
            Primitive::Lines => vk::PrimitiveTopology::LINE_LIST,
            Primitive::Triangles => vk::PrimitiveTopology::TRIANGLE_LIST,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct SceneData {
    cameras: [f32; 4 * 4 * 2],
    anim: f32,
}

unsafe impl bytemuck::Zeroable for SceneData {}
unsafe impl bytemuck::Pod for SceneData {}

impl MainLoop<DrawList> for App {
    fn new(core: &SharedCore, mut platform: Platform<'_>, draw_data: DrawList) -> Result<Self> {
        let settings = Settings {
            msaa_samples: 4
        };
        let mut starter_kit = StarterKit::new(core.clone(), &mut platform, settings)?;

        // Camera
        let camera = MultiPlatformCamera::new(&mut platform);

        // Scene data
        let scene_ubo = FrameDataUbo::new(core.clone(), FRAMES_IN_FLIGHT)?;

        // Create descriptor set layout
        const FRAME_DATA_BINDING: u32 = 0;
        let bindings = [
            vk::DescriptorSetLayoutBindingBuilder::new()
                .binding(FRAME_DATA_BINDING)
                .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::ALL_GRAPHICS),
        ];

        let descriptor_set_layout_ci =
            vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

        let descriptor_set_layout = unsafe {
            core.device
                .create_descriptor_set_layout(&descriptor_set_layout_ci, None, None)
        }
        .result()?;

        // Create descriptor pool
        let pool_sizes = [
            vk::DescriptorPoolSizeBuilder::new()
                ._type(vk::DescriptorType::UNIFORM_BUFFER)
                .descriptor_count(FRAMES_IN_FLIGHT as _),
        ];

        let create_info = vk::DescriptorPoolCreateInfoBuilder::new()
            .pool_sizes(&pool_sizes)
            .max_sets((FRAMES_IN_FLIGHT * 2) as _);

        let descriptor_pool =
            unsafe { core.device.create_descriptor_pool(&create_info, None, None) }.result()?;

        // Create descriptor sets
        let layouts = vec![descriptor_set_layout; FRAMES_IN_FLIGHT];
        let create_info = vk::DescriptorSetAllocateInfoBuilder::new()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets =
            unsafe { core.device.allocate_descriptor_sets(&create_info) }.result()?;

        // Write descriptor sets
        for (frame, &descriptor_set) in descriptor_sets.iter().enumerate() {
            let frame_data_bi = [scene_ubo.descriptor_buffer_info(frame)];
            let writes = [
                vk::WriteDescriptorSetBuilder::new()
                    .buffer_info(&frame_data_bi)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .dst_set(descriptor_set)
                    .dst_binding(FRAME_DATA_BINDING)
                    .dst_array_element(0),
            ];

            unsafe {
                core.device.update_descriptor_sets(&writes, &[]);
            }
        }


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

        // Pipelines
        let unlit_vert = include_bytes!("../shaders/unlit.vert.spv");
        let unlit_frag = include_bytes!("../shaders/unlit.frag.spv");

        let point_pipeline = shader(
            core,
            unlit_vert,
            unlit_frag,
            Primitive::Points.into(),
            starter_kit.render_pass,
            pipeline_layout,
            starter_kit.msaa_samples
        )?;

        let line_pipeline = shader(
            core,
            unlit_vert,
            unlit_frag,
            Primitive::Lines.into(),
            starter_kit.render_pass,
            pipeline_layout,
            starter_kit.msaa_samples
        )?;

        let tri_pipeline = shader(
            core,
            unlit_vert,
            unlit_frag,
            Primitive::Triangles.into(),
            starter_kit.render_pass,
            pipeline_layout,
            starter_kit.msaa_samples
        )?;

        // Mesh uploads
        let mut draw = vec![];
        for data in draw_data {
            let mesh = upload_mesh(
                &mut starter_kit.staging_buffer,
                starter_kit.command_buffers[0],
                &data.vertices,
                &data.indices,
            )?;
            draw.push((mesh, data.primitive));
        }

        Ok(Self {
            camera,
            descriptor_set_layout,
            descriptor_sets,
            descriptor_pool,
            anim: 0.0,
            pipeline_layout,
            scene_ubo,
            draw,
            point_pipeline,
            line_pipeline,
            tri_pipeline,
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
                &[self.descriptor_sets[self.starter_kit.frame]],
                &[],
            );

            // Draw cmds
            for filter in [Primitive::Points, Primitive::Lines, Primitive::Triangles] {
                core.device.cmd_bind_pipeline(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    match filter {
                        Primitive::Points => self.point_pipeline,
                        Primitive::Lines => self.line_pipeline,
                        Primitive::Triangles => self.tri_pipeline,
                    }
                );

                for (mesh, primitive) in &self.draw {
                    if *primitive == filter {
                        draw_mesh(core, command_buffer, &mesh);
                    }
                }
            }
        }

        let (ret, cameras) = self.camera.get_matrices(&platform)?;

        self.scene_ubo.upload(
            self.starter_kit.frame,
            &SceneData {
                cameras,
                anim: self.anim,
            },
        )?;

        self.anim += 1.0;

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
        _core: &Core,
        mut platform: Platform<'_>,
    ) -> Result<()> {
        self.camera.handle_event(&mut event, &mut platform);
        starter_kit::close_when_asked(event, platform);
        Ok(())
    }
}

impl SyncMainLoop<DrawList> for App {
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.starter_kit.winit_sync()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            self.starter_kit.core.device.device_wait_idle().unwrap();
            self.starter_kit.core.device.destroy_descriptor_pool(Some(self.descriptor_pool), None);
            self.starter_kit.core.device.destroy_descriptor_set_layout(Some(self.descriptor_set_layout), None);
            self.starter_kit.core.device.destroy_pipeline_layout(Some(self.pipeline_layout), None);
            for pipeline in [self.tri_pipeline, self.line_pipeline, self.point_pipeline] {
                self.starter_kit.core.device.destroy_pipeline(Some(pipeline), None);
            }
        }
    }
}
