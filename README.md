# LumeVisual

LumeVisual is a high-level, pure Rust rendering library designed for the next generation of real-time graphics (Ray Tracing & Virtual Geometry).

## Philosophy
- **First Principles**: Re-evaluating the graphics pipeline for modern hardware.
- **Ray Tracing Native**: No legacy rasterization support.
- **Virtual Geometry**: Built-in support for "LOD-less" rendering via **LumeAdaptrix**.
- **Explicit Backends**: Vulkan 1.3+ and Metal 3.0+ only.

## Structure
- `lume-core`: Main API and type definitions.
- `lume-vulkan`: Vulkan backend implementation.
- `lume-metal`: Metal backend implementation.
- `lume-adaptrix`: Virtual Geometry and streaming system.

### Current Status
- **lume-vulkan**: Fully functional core for resource management (Buffers, Textures), modern BindGroup-based descriptor management, and synchronized frame loop.
- **Examples**: `hello_triangle` now renders a fully textured and transformed 3D cube with depth testing.
- **Stability**: Fixed critical driver-level memory access issues and ensured safe GPU shutdown.
