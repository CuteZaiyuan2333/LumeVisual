#version 450

layout(location = 0) in vec2 inTexCoord;
layout(location = 0) out vec4 outColor;

layout(set = 0, binding = 1) uniform texture2D t_diffuse;
layout(set = 0, binding = 2) uniform sampler s_diffuse;

void main() {
    outColor = texture(sampler2D(t_diffuse, s_diffuse), inTexCoord);
}
