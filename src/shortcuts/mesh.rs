use erupt::vk;
use crate::shortcuts::{ManagedBuffer, StagingBuffer, Vertex};
use crate::Core;
use anyhow::Result;

pub fn upload_mesh(
    staging: &mut StagingBuffer,
    command_buffer: vk::CommandBuffer,
    vertices: &[Vertex],
    indices: &[u32],
) -> Result<ManagedMesh> {
    let n_indices = indices.len() as u32;

    let vertex_ci = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .size(std::mem::size_of_val(vertices) as u64);

    let index_ci = vk::BufferCreateInfoBuilder::new()
            .usage(vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .size(std::mem::size_of_val(indices) as u64);


    let vertices = staging.upload(command_buffer, vertex_ci, &vertices)?;
    let indices = staging.upload(command_buffer, index_ci, &indices)?;
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

pub fn draw_meshes(core: &Core, command_buffer: vk::CommandBuffer, meshes: &[ManagedMesh]) {
    for mesh in meshes {
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
            core.device
                .cmd_draw_indexed(command_buffer, mesh.n_indices, 1, 0, 0, 0);
        }
    }
}
