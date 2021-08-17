
#version 450
#extension GL_ARB_separate_shader_objects : enable
#extension GL_EXT_multiview : require

layout(binding = 0) uniform Animation {
    mat4 camera[2];
    float anim;
};

/*
layout(binding = 0) uniform CameraUbo {
};

layout(push_constant) uniform Model {
    mat4 model;
};
*/

layout(location = 0) in vec3 inPosition;
layout(location = 1) in vec3 inColor;

layout(location = 0) out vec3 fragColor;
layout(location = 1) out float vertidx;

void main() {
    //gl_Position = camera[gl_ViewIndex] * model * vec4(inPosition, 1.0);
    vec4 glpos = camera[gl_ViewIndex] * vec4(inPosition, 1.0);
    gl_Position = glpos;
    gl_PointSize = clamp((1. - glpos.z) * 5., 1., 10.);
    fragColor = inColor;
    vertidx = float(gl_VertexIndex);
}

