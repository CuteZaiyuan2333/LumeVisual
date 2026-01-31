# LumeVisual Design & Roadmap

## Core Vision
LumeVisual is a modern, high-performance rendering library written in Rust. It aims to bridge the gap between low-level hardware control (Vulkan/Metal) and high-level game engine needs.

### Key Goals
- **Ray Tracing Native**: Designed from the ground up for Hardware Ray Tracing.
- **Virtual Geometry**: Integrated support for massive asset streaming (inspired by Nanite).
- **Static Backend**: Use of Rust's trait system for zero-overhead abstraction.

## Architecture
The system is divided into several crates:
- `lume-core`: Common traits and types. Backend agnostic.
- `lume-vulkan`: Vulkan 1.3+ backend implementation.
- `lume-metal`: Metal backend implementation (macOS/iOS).
- `lume-adaptrix`: The virtual geometry and streaming engine.

## Current Progress: Phase 2 (Core Loop) - [COMPLETED]
We have established the foundational graphics objects:
1.  **Instance & Surface**: [Completed] Initialized and connected via Winit 0.30.
2.  **Device Selection**: [Completed] Stable GPU selection logic.
3.  **Swapchain**: [Completed] Robust presentation engine with explicit synchronization.
4.  **Graphics Pipeline**: [Completed] Modern descriptor-based (BindGroup) pipeline.
5.  **Commands & Rendering**: [Completed] Fully functional command recording and submission.
6.  **Resource Management**: [Completed] Buffer (Vertex/Uniform) and Texture (SAMPLED_IMAGE) management via `gpu-allocator`.
7.  **Uniforms & Descriptors**: [Completed] Stable bind group updates with pointer safety.

## Future Plans (Phase 3+)
- **Resource Management**: Buffer and Texture allocation (VMA integration).
- **Command Buffers**: High-level command recording API.
- **Lume-RenderGraph**: A graph-based deferred/clustered renderer.
- **Adaptrix Integration**: First prototypes of mesh shading and streaming.
