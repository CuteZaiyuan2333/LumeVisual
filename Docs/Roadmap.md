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

## Current Progress: Phase 2 (Core Loop)
We are currently implementing the foundational graphics objects:
1.  **Instance & Surface**: Initialized and connected to the OS window (Winit 0.30).
2.  **Device Selection**: Picking the best GPU (Intel Arc B580 detected).
3.  **Swapchain**: [Completed] Managing images for screen presentation.
4.  **Graphics Pipeline**: [Completed] Shaders and pipeline state initialized.
5.  **Commands & Rendering**: [Completed] Command buffers and frame loop functional.
6.  **Resource Management**: [Completed] Vertex Buffers and memory allocation.
7.  **Uniforms & Descriptors**: [In Progress] Binding global data to shaders.

## Future Plans (Phase 3+)
- **Resource Management**: Buffer and Texture allocation (VMA integration).
- **Command Buffers**: High-level command recording API.
- **Lume-RenderGraph**: A graph-based deferred/clustered renderer.
- **Adaptrix Integration**: First prototypes of mesh shading and streaming.
