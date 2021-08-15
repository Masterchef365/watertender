#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragColor;
layout(location = 1) in float vertidx;

layout(binding = 0) uniform Animation {
    mat4 camera[2];
    float anim;
};

layout(location = 0) out vec4 outColor;

void main() {
    bool m = fract(vertidx / 500. - anim / 1000.) < 0.5;
    if (m) discard;
    outColor = vec4(fragColor, 1.0);
}
