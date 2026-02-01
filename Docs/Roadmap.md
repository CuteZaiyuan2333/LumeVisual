# LumeVisual 核心路线图：次世代虚拟化渲染

## 核心愿景
LumeVisual 是一款剔除传统光栅化与 LOD 逻辑的现代引擎。
- **全虚拟化几何 (LOD-less)**: 基于 Adaptrix 的微多边形流送。
- **动态全局光照 (Lumen-like)**: 基于 Surface Cache 与硬件光追的实时 GI。
- **GPU 驱动架构**: 100% GPU 驱动的流水线，完全消除 CPU 提交瓶颈。

## 开发阶段

### 阶段 1: 基础设施 (已完成)
- [x] Vulkan 1.3 现代后端封装。
- [x] Bindless 资源索引 (10万级纹理/缓存支持)。
- [x] 自动帧并行同步 (Frame-in-Flight)。

### 阶段 2: Adaptrix 虚拟几何系统 (当前核心)
- [ ] **几何集群化 (Clustering)**: 离线将 Mesh 划分为 128-256 顶点的集群。
- [ ] **GPU 驱动剔除**: 基于 Compute Shader 的两级剔除（Instance & Cluster）。
- [ ] **可见性缓冲 (VisBuffer)**: 实现高密度微多边形的 ID 渲染。
- [ ] **虚拟化流送 (Streaming)**: 基于相机视锥的动态数据加载与 LRU 缓存。

### 阶段 3: Lume-GI 光照系统 (计划中)
- [ ] **表面缓存 (Surface Cache)**: 为虚拟几何集群生成并展开光照贴图缓存。
- [ ] **硬件光追集成**: 使用 RT Core 加速间接光探测。
- [ ] **辐射度传播**: 实现基于探针或漫反射路径追踪的全局光照。

### 阶段 4: 后处理与集成
- [ ] 时域超采样 (Temporal Super Resolution)。
- [ ] 材质图元系统。