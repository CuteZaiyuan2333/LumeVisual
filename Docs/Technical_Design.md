# LumeVisual 技术设计方案

## 1. 渲染哲学：弃绝传统光栅化
LumeVisual 不再支持 `vkCmdDrawIndexed` 渲染成千上万个小 Mesh 的传统模式，也不要求美术制作 LOD。
- **唯一输入**: 高精度原始模型。
- **唯一输出**: 由 Adaptrix 系统动态生成的可见像素。

## 2. 关键系统架构

### A. Adaptrix (虚拟几何系统)
- **数据结构**: 几何体被切分为 Cluster。每个 Cluster 包含边界球（Bounding Sphere）和误差度量（Error Metric）。
- **渲染管线**: 
  - 使用 Compute Shader 进行剔除。
  - 使用 Mesh Shader (或优化后的多重绘制) 渲染 ID 到 VisBuffer。
- **去 LOD 化**: 系统根据屏幕误差自动选择合适的 Cluster 级别，实现无缝连续 LOD。

### B. Lume-GI (全局光照)
- **解耦光照**: 光照计算在空间（Surface Cache）而非像素空间进行。
- **混合追踪**:
  - 近场: 硬件光线追踪 (VK_KHR_ray_tracing)。
  - 远场: 高度场/SDF 或简化几何代理。
- **反馈闭环**: 只有被 Adaptrix 标记为可见的 Cluster 才会触发 Surface Cache 的更新。

## 3. 待办技术债
- [ ] **移除旧 API**: 彻底移除 `lume-vulkan` 中关于传统 RenderPass 的旧逻辑，全面转向单 Pass VisBuffer 架构。
- [ ] **数据转换工具**: 需要编写一个处理 `.obj/.gltf` 并生成 Adaptrix 专有格式的工具。
