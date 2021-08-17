#version 450
#extension GL_ARB_separate_shader_objects : enable

layout(location = 0) in vec3 fragColor;
layout(location = 1) in float vertidx;

layout(binding = 0) uniform Animation {
    mat4 camera[2];
    float anim;
};

layout(location = 0) out vec4 outColor;

/*
vec3 hsv2rgb(vec3 c) {
  vec4 K = vec4(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
  vec3 p = abs(fract(c.xxx + K.xyz) * 6.0 - K.www);
  return c.z * mix(K.xxx, clamp(p - K.xxx, 0.0, 1.0), c.y);
}
*/

float rand(vec3 co){
    return fract(sin(dot(co, vec3(12.9898,78.233,43.15214))) * 43758.5453);
}

void main() {
    bool m = fract(vertidx / 500. - anim / 1000. + cos(vertidx / 13.231) / 1000.) < 0.5;
    if (m) discard;
    float q = fragColor.x / 9.;
    //vec3 color = vec3(mix(vec3(0.016,0.543,1.000), vec3(0.08,0.07,0.010), pow(abs(q), 1.664)));
    //vec3 color = vec3(mix(vec3(0.08,0.07,0.010), vec3(0.016,0.543,1.000), q + 0.1));
    //vec3 color = vec3(mix(vec3(0.011,0.067,0.080), vec3(0.016,0.543,1.000), q + 0.184));
    vec3 color = vec3(mix(vec3(0.), vec3(0.016,0.543,1.000), pow(q, 3.)));
    color *= (cos(vertidx / 12.234234) + 1.0 + 0.03) * 0.8;

    outColor = vec4(
        color,
        1.0
    );
}
