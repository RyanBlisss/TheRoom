#version 330 core

in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoord;

uniform vec3  lightPos;       // overhead ceiling lamp
uniform vec3  eyePos;         // player eye — local ambient glow
uniform vec3  lightColor;
uniform vec3  objectColor;
uniform float ambientStrength;
uniform float sanity;

out vec4 FragColor;

// Attenuated point light contribution (diffuse only)
vec3 pointLight(vec3 lPos, vec3 lColor, float strength) {
    vec3  norm     = normalize(Normal);
    vec3  dir      = normalize(lPos - FragPos);
    float diff     = max(dot(norm, dir), 0.0);
    float dist     = length(lPos - FragPos);
    float atten    = 1.0 / (1.0 + 0.30 * dist + 0.18 * dist * dist);
    return diff * lColor * strength * atten;
}

void main() {
    // Base ambient (very low — rooms should feel dark)
    vec3 ambient = ambientStrength * lightColor;

    // Overhead warm incandescent lamp in the lobby
    vec3 overhead = pointLight(lightPos, vec3(1.0, 0.88, 0.68), 3.5);

    // Soft proximity glow from the player — lights up ~4m radius around them
    vec3 nearby = pointLight(eyePos, vec3(1.0, 0.97, 0.92), 1.4);

    vec3 result = (ambient + overhead + nearby) * objectColor;

    // Sanity: desaturate and push toward cold purple-grey as sanity drops
    float insanity = 1.0 - sanity;
    float lum = dot(result, vec3(0.299, 0.587, 0.114));
    result = mix(result, vec3(lum) * vec3(0.58, 0.50, 0.72), insanity * 0.60);

    FragColor = vec4(result, 1.0);
}
