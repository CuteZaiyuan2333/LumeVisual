#version 450

layout(location = 0) in flat uint inClusterID;
layout(location = 1) in flat uint inPrimitiveID;

layout(location = 0) out uvec2 outVisData; // R32G32_UINT

void main() {
    // 写入可见性数据
    // R: ClusterID
    // G: PrimitiveID (或者合并 InstanceID)
    outVisData = uvec2(inClusterID, inPrimitiveID);
}
