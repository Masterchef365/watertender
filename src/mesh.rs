use crate::{memory::ManagedBuffer, staging_buffer::StagingBuffer, vertex::Vertex};
use crate::Core;
use anyhow::Result;
use erupt::vk;

pub fn upload_mesh(
    staging: &mut StagingBuffer,
    command_buffer: vk::CommandBuffer,
    vertices: &[Vertex],
    indices: &[u32],
) -> Result<ManagedMesh> {
    let n_indices = indices.len() as u32;

    let vertices = staging.upload_buffer_pod(
        command_buffer,
        vk::BufferUsageFlags::VERTEX_BUFFER,
        &vertices,
    )?;
    let indices =
        staging.upload_buffer_pod(command_buffer, vk::BufferUsageFlags::INDEX_BUFFER, &indices)?;
    Ok(ManagedMesh {
        vertices,
        indices,
        n_indices,
    })
}

pub struct ManagedMesh {
    pub vertices: ManagedBuffer,
    pub indices: ManagedBuffer,
    pub n_indices: u32,
}

pub fn draw_mesh(core: &Core, command_buffer: vk::CommandBuffer, mesh: &ManagedMesh) {
    unsafe {
        core.device.cmd_bind_vertex_buffers(
            command_buffer,
            0,
            &[mesh.vertices.instance()],
            &[0],
        );
        core.device.cmd_bind_index_buffer(
            command_buffer,
            mesh.indices.instance(),
            0,
            vk::IndexType::UINT32,
        );
        core.device.cmd_draw_indexed(command_buffer, mesh.n_indices, 1, 0, 0, 0);
    }
}
