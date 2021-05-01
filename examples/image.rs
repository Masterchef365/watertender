use anyhow::{Context, Result};
use shortcuts::{
    launch,
    memory::ManagedImage,
    mesh::*,
    shader,
    starter_kit::{self, StarterKit},
    FrameDataUbo, MultiPlatformCamera, StagingBuffer, Vertex,
};
use std::path::Path;
use watertender::*;

struct App {
    descriptor_set: vk::DescriptorSet,
    cube_tex: ManagedImage,
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

const TEXTURE_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

impl MainLoop for App {
    fn new(core: &SharedCore, mut platform: Platform<'_>) -> Result<Self> {
        let mut starter_kit = StarterKit::new(core.clone(), &mut platform)?;

        // Camera
        let camera = MultiPlatformCamera::new(&mut platform);

        // Descriptor set
        let bindings = [vk::DescriptorSetLayoutBindingBuilder::new()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)];
        let descriptor_set_layout_ci =
            vk::DescriptorSetLayoutCreateInfoBuilder::new().bindings(&bindings);

        let descriptor_set_layout = unsafe {
            core.device
                .create_descriptor_set_layout(&descriptor_set_layout_ci, None, None)
        }
        .result()?;

        // Create descriptor pool
        let pool_sizes = [vk::DescriptorPoolSizeBuilder::new()
            ._type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(1)];

        let create_info = vk::DescriptorPoolCreateInfoBuilder::new()
            .pool_sizes(&pool_sizes)
            .max_sets(1);

        let descriptor_pool =
            unsafe { core.device.create_descriptor_pool(&create_info, None, None) }.result()?;

        // Create descriptor sets
        let layouts = vec![descriptor_set_layout];
        let create_info = vk::DescriptorSetAllocateInfoBuilder::new()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_set =
            unsafe { core.device.allocate_descriptor_sets(&create_info) }.result()?[0];

        // Scene data
        let scene_ubo = FrameDataUbo::new(
            core.clone(),
            starter_kit::FRAMES_IN_FLIGHT,
            0, // This takes a WHOLE descriptr set. h.
        )?;

        let descriptor_set_layouts = [scene_ubo.descriptor_set_layout(), descriptor_set_layout];

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
            include_bytes!("unlit_tex.frag.spv"),
            vk::PrimitiveTopology::TRIANGLE_LIST,
            starter_kit.render_pass,
            pipeline_layout,
        )?;

        // Mesh uploads
        let (vertices, indices) = rainbow_cube();
        let rainbow_cube = upload_mesh(
            &mut starter_kit.staging_buffer,
            starter_kit.command_buffers[0],
            &vertices,
            &indices,
        )?;

        // Image uploads
        let image_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        let command_buffer = starter_kit.current_command_buffer(); // TODO: This probably breaks stuff lmaoo
        let (cube_tex, image_sub) = upload_image(
            command_buffer,
            &mut starter_kit.staging_buffer,
            image_layout,
            "./examples/obama.png",
        )?;

        // Create image view
        let create_info = vk::ImageViewCreateInfoBuilder::new()
            .image(cube_tex.instance())
            .view_type(vk::ImageViewType::_2D)
            .format(TEXTURE_FORMAT)
            .subresource_range(image_sub.build())
            .build();

        let image_view =
            unsafe { core.device.create_image_view(&create_info, None, None) }.result()?;

        // Create sampler
        let create_info = vk::SamplerCreateInfoBuilder::new()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::REPEAT)
            .address_mode_v(vk::SamplerAddressMode::REPEAT)
            .address_mode_w(vk::SamplerAddressMode::REPEAT)
            .anisotropy_enable(false)
            .max_anisotropy(16.)
            .border_color(vk::BorderColor::INT_OPAQUE_BLACK)
            .unnormalized_coordinates(false)
            .compare_enable(false)
            .compare_op(vk::CompareOp::ALWAYS)
            .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
            .mip_lod_bias(0.)
            .min_lod(0.)
            .max_lod(0.)
            .build();

        let sampler = unsafe { core.device.create_sampler(&create_info, None, None) }.result()?;

        // Descriptor write
        let image_infos = [vk::DescriptorImageInfoBuilder::new()
            .image_layout(image_layout)
            .image_view(image_view)
            .sampler(sampler)];

        let writes = [vk::WriteDescriptorSetBuilder::new()
            .image_info(&image_infos)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)];

        unsafe {
            core.device.update_descriptor_sets(&writes, &[]);
        }

        Ok(Self {
            descriptor_set,
            cube_tex,
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
                &[
                    self.scene_ubo.descriptor_set(self.starter_kit.frame),
                    self.descriptor_set,
                ],
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
        )?;

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

impl SyncMainLoop for App {
    fn winit_sync(&self) -> (vk::Semaphore, vk::Semaphore) {
        self.starter_kit.winit_sync()
    }
}

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

fn upload_image(
    command_buffer: vk::CommandBuffer,
    staging_buffer: &mut StagingBuffer,
    final_layout: vk::ImageLayout,
    path: impl AsRef<Path>,
) -> Result<(ManagedImage, vk::ImageSubresourceRangeBuilder<'static>)> {
    let (image, info) = read_image(path).context("Failed to read image")?;

    let extent = vk::Extent3DBuilder::new()
        .width(info.width)
        .height(info.height)
        .depth(1)
        .build();

    let create_info = vk::ImageCreateInfoBuilder::new()
        .image_type(vk::ImageType::_2D)
        .extent(extent)
        .mip_levels(1)
        .array_layers(1)
        .format(TEXTURE_FORMAT)
        .tiling(vk::ImageTiling::OPTIMAL)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .samples(vk::SampleCountFlagBits::_1);

    let offset = vk::Offset3DBuilder::new().x(0).y(0).z(0).build();

    let image_subresources = vk::ImageSubresourceLayersBuilder::new()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .mip_level(0)
        .base_array_layer(0)
        .layer_count(1)
        .build();

    let subresource_range = vk::ImageSubresourceRangeBuilder::new()
        .aspect_mask(image_subresources.aspect_mask)
        .base_mip_level(image_subresources.mip_level)
        .level_count(1)
        .base_array_layer(image_subresources.base_array_layer)
        .layer_count(image_subresources.layer_count);

    let copy = vk::BufferImageCopyBuilder::new()
        .buffer_offset(0)
        .buffer_row_length(0)
        .buffer_image_height(0)
        .image_subresource(image_subresources)
        .image_offset(offset)
        .image_extent(extent);

    let image = staging_buffer.upload_image(
        command_buffer,
        create_info,
        copy,
        subresource_range,
        final_layout,
        &image,
    )?;

    Ok((image, subresource_range))
}

fn read_image(path: impl AsRef<Path>) -> Result<(Vec<u8>, png::OutputInfo)> {
    let img = png::Decoder::new(std::fs::File::open(path)?);
    let (info, mut reader) = img.read_info()?;

    assert!(info.color_type == png::ColorType::RGBA);
    assert!(info.bit_depth == png::BitDepth::Eight);

    let mut img_buffer = vec![0; info.buffer_size()];

    assert_eq!(info.buffer_size(), (info.width * info.height * 4) as _);
    reader.next_frame(&mut img_buffer)?;

    Ok((img_buffer, info))
}