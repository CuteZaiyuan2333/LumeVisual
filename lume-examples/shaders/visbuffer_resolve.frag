#version 450
#extension GL_EXT_samplerless_texture_functions : require

layout(set = 0, binding = 0) uniform utexture2D visBuffer; 

layout(location = 0) in vec2 inTexCoord;
layout(location = 0) out vec4 outColor;

vec3 hash3(uint n) {
    n = (n << 13U) ^ n;
    n = n * (n * n * 15731U + 789221U) + 1376312589U;
    vec3 f = vec3(n & uvec3(0x09247440U, 0x00804020U, 0x24020080U)) / float(0x40000000);
    return f;
}

void main() {
    ivec2 texelCoord = ivec2(gl_FragCoord.xy);
    uint clusterID = texelFetch(visBuffer, texelCoord, 0).r;

    if (clusterID == 0xFFFFFFFF) {
        outColor = vec4(0.1, 0.1, 0.1, 1.0);
    } else {
        vec3 color = hash3(clusterID);
        outColor = vec4(color, 1.0);
    }
}
