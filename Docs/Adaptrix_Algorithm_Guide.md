# Adaptrix 虚拟几何体系统：算法与实现指南

本指南总结了 `bevy` 与 `UE5 Nanite` 的核心实现，作为 `lume-adaptrix` 模块的开发基准。

## 1. 离线预处理流水线 (The Processor)
不再使用原始索引缓冲区，模型必须被转换为 `AdaptrixMesh` 格式。

### A. 集群划分 (Meshlet Partitioning)
- **目标**: 将 Mesh 划分为固定大小的 `Cluster`（例如：最多 128 个顶点，256 个三角形）。
- **算法**: 使用 METIS 算法或图形分区算法，确保 Cluster 之间的边界共享尽可能少的顶点。
- **数据结构**:
  ```rust
  struct Cluster {
      vertex_offset: u32,
      triangle_offset: u32,
      vertex_count: u8,
      triangle_count: u8,
      bounding_sphere: [f32; 4], // x, y, z, radius
      error_metric: f32,         // 几何简化误差
      parent_error: f32,         // 父节点的误差（用于层级选择）
  }
  ```

### B. 层级构建 (LOD DAG/Tree)
- **合并与简化**: 将相邻的 Clusters 合并，通过边折叠（Edge Collapse）算法简化三角形，再重新划分为新的 Clusters。
- **误差度量**: 记录简化导致的几何偏差。只有当屏幕像素误差（Projected Error）小于阈值时，才允许显示当前层级。

## 2. GPU 渲染流水线 (The Pipeline)

### 第一阶段：两级剔除 (Two-Pass Culling)
1.  **Instance Culling**: 剔除完全在视锥外的物体。
2.  **Cluster Culling**: 
    - **视锥剔除**: 检查 Cluster 边界球。
    - **层级选择**: 检查 `error_metric`。如果当前 Cluster 的投影误差足够小，且其父节点的投影误差太大，则选择该 Cluster。
    - **遮挡剔除 (HZB)**: 使用上一帧生成的 HZB 剔除被遮挡的集群。

### 第二阶段：可见性缓冲 (Visibility Buffer)
- **极简光栅化**: 
  - 硬件路径：使用 `Mesh Shader` 或 `MultiDrawIndirect`。
  - 软件路径（针对极小三角形）：在 Compute Shader 中手动原子操作写入。
- **输出内容**: 
  - `R64_UINT` 或 `R32G32_UINT` 纹理。
  - 存储格式：`u32 InstanceID | u32 ClusterID | u10 PrimitiveID`。

### 第三阶段：材质解析 (Material Resolve)
- **全屏 Pass**: 遍历 VisBuffer。
- **属性插值**: 根据 `PrimitiveID` 从存储缓冲区（Bindless Buffers）中读取顶点数据，手动进行重心坐标插值（Barycentric Interpolation）得到 UV、法线等。
- **Surface Cache 交互**: 将可见的集群坐标映射到 Surface Cache 进行 GI 计算。

## 3. 关键性能点
- **Bindless Everything**: 所有的顶点、索引、材质参数都在全局 Descriptor Set 中。
- **Persistent Threads**: 剔除逻辑在单个 Compute Shader 中通过全局队列（Global Queue）完成，避免 CPU 与 GPU 之间的反复同步。
- **64-bit Atomics**: 使用 `imageAtomicMax` 确保 VisBuffer 的深度测试在原子操作中完成。

## 4. 与 Lume-GI 的集成
- 只有被 VisBuffer 标记为可见的像素所属的 Cluster，才会在 Surface Cache 中分配空间。
- GI 系统通过查询 VisBuffer 快速定位需要二次反射计算的表面。

  规划建议：                                                       
                                                                         
   1. 启动 Adaptrix 离线处理工具：这是目前最迫切的。建议开发一个名为     
      lume-processor 的独立小工具，引用 meshopt 库，负责将 .obj          
      模型切碎成文档中描述的 Cluster 格式。                              
   2. 定义 `lume-adaptrix` 核心 trait：基于文档，在                      
      lume-adaptrix/src/lib.rs 中定义 GPU 端的布局（Layout）。           
   3. 开发 GPU 剔除原型：在 lume-examples 中增加一个                     
      test_culling.rs，模拟大规模 Cluster 提交，并验证 Compute Shader    
      剔除的正确性。
