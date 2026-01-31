# LumeVisual: Technical Handoff & Virtual Geometry Guide

## Architecture Overview
LumeVisual is a trait-based rendering engine.
- `lume-core`: Defines the protocol. Backend agnostic.
- `lume-vulkan`: Vulkan 1.3 implementation. Uses `ash` and `gpu-allocator`.
- `lume-adaptrix`: **[Target for next phase]** This module will implement Virtual Geometry.

## Current Robustness Status
1. **Synchronization**: Explicit sync is implemented via Semaphores. The example now uses `wait_idle()` per frame to ensure stability. **Next Step**: Implement a Frame-in-Flight system with Fences to improve performance.
2. **Resource Management**: Buffers and Textures are managed via `gpu-allocator`. Uniform Buffers require alignment (usually 64 or 256 bytes depending on GPU).
3. **Descriptor Management**: Moved to `lume-vulkan/src/descriptor.rs`. Uses a WebGPU-like BindGroup abstraction.
4. **Safety**: Fixed critical pointer aliasing in `update_descriptor_sets`.

## For the next AI (Virtual Geometry Developer):
### High-Level Goal
Implement a Nanite-like streaming system in `lume-adaptrix`.

### Technical Prerequisites
1. **Mesh Shading**: You need to enable `VK_EXT_mesh_shader`. Check device features in `instance.rs`.
2. **GPU Data Structures**: Virtual geometry relies on large `StorageBuffer`s (SSBOs) for cluster data and visibility buffers. 
3. **Async Compute**: Adaptrix should ideally use a separate compute queue for cluster culling.
4. **Bindless Textures**: For massive asset streaming, you must implement Bindless Descriptors (`VK_EXT_descriptor_indexing`).

### Known Technical Debt / Implementation Notes
- **Texture Views**: Currently, `VulkanTextureView` only supports basic 2D views. Cubemaps/Arrays need extension.
- **Error Handling**: Many `.expect()` calls remain in setup code. Consider converting to `LumeResult`.
- **Memory Aliasing**: The `create_bind_group` implementation uses a stable-pointer workaround. It works but could be more idiomatic using a specialized arena.

## Getting Started with Adaptrix
1. Define `Cluster` and `Meshlet` structures in `lume-adaptrix`.
2. Implement a GPU-driven culling pass using Compute Shaders.
3. Integrate with the main `lume-vulkan` pipeline via `StorageBuffer` bindings.

---
*Signed by: LumeVisual Bootstrap Agent (2026-02-01)*