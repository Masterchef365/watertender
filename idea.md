# Watertender

## API
```rust
struct Frame {
    index: usize,
    serial_cmds: vk::CommandBuffer,
    frame_cmds: vk::CommandBuffer,
    framebuffer: vk::FrameBuffer,
    extent: vk::Extent2D,
}
```

```rust
trait MainLoop {
    fn new(init_cmds: vk::CommandBuffer, core: &Core, platform: Platform<'a>) -> Result<Self>;
    fn event(&mut self, event: PlatformEvent, core: &Core, platform: Platform<'a>) -> Result<()>;
    fn frame(&mut self, frame: Frame, core: &Core, platform: Platform<'a>) -> Result<()>;
}
```

Watertender supplies:
* A multi-platform main loop coupled with:
    * multi-platform Swapchain
        * Handles resizes
    * Simple synchronization
    * Core (queue creation, etc.)
    * A slightly easier way to allocate resources than the gpu-alloc crate
* A Bunch of pre-built renderer parts (Things for rendering meshes!)
    * These are mostly just dumb shortcuts, but there may be some more advanced modules (such as auto-sync)
    * Cameras, platform-agnostic abstractions _over_ cameras

## Goals
* Flexibility through __ease of modification__ over generality
    * Vulkan is extremely versatile and fitting all use-cases would require Vulkan itself
* Faster time-to-start for GPU programming

## Ideas
* Maybe many of the shortcuts are just traits implemented on &Core... 
    * This would necessitate that the user imports the traits, but it would be nice and easy

