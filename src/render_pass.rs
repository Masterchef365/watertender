use crate::defaults::{COLOR_FORMAT, DEPTH_FORMAT};
use crate::Core;
use anyhow::Result;
use erupt::{vk, vk1_1};

pub fn create_render_pass(core: &Core, vr: bool, msaa_samples: vk::SampleCountFlagBits) -> Result<vk::RenderPass> {
    let device = &core.device;

    // Render pass
    let color_attachment = vk::AttachmentDescriptionBuilder::new()
        .samples(msaa_samples)
        .format(COLOR_FORMAT)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let depth_attachment = vk::AttachmentDescriptionBuilder::new()
        .samples(msaa_samples)
        .format(DEPTH_FORMAT)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let resolve_attachment = vk::AttachmentDescriptionBuilder::new()
        .format(COLOR_FORMAT)
        .samples(vk::SampleCountFlagBits::_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(if vr {
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
        } else {
            vk::ImageLayout::PRESENT_SRC_KHR
        });

    let attachments = [color_attachment, depth_attachment, resolve_attachment];

    let color_attachment_refs = [vk::AttachmentReferenceBuilder::new()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

    let depth_attachment_ref = vk::AttachmentReferenceBuilder::new()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .build();

    let resolve_attachment_refs = [vk::AttachmentReferenceBuilder::new()
        .attachment(2)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)];

    let subpasses = [vk::SubpassDescriptionBuilder::new()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachment_refs)
        .depth_stencil_attachment(&depth_attachment_ref)
        .resolve_attachments(&resolve_attachment_refs)];

    let dependencies = [vk::SubpassDependencyBuilder::new()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
        .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)];

    let mut create_info = vk::RenderPassCreateInfoBuilder::new()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    let views = if vr { 2 } else { 1 };
    let view_mask = [!(!0 << views)];
    let mut multiview = vk1_1::RenderPassMultiviewCreateInfoBuilder::new()
        .view_masks(&view_mask)
        .correlation_masks(&view_mask)
        .build();

    create_info.p_next = &mut multiview as *mut _ as _;

    Ok(unsafe { device.create_render_pass(&create_info, None, None) }.result()?)
}
