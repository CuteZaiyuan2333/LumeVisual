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

## Status
Planning & Skeleton Phase.
See `LumeVisual_Architecture.md` for design details.
