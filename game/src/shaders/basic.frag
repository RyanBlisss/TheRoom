#version 330 core

in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoord;

uniform vec3  lightPos;
uniform vec3  lightColor;
uniform vec3  objectColor;
uniform float ambientStrength;
uniform float sanity;        // 0.0 - 1.0, affects color grading

out vec4 FragColor;

void main() {
    // Ambient
    vec3 ambient = ambientStrength * lightColor;

    // Diffuse
    vec3 norm     = normalize(Normal);
    vec3 lightDir = normalize(lightPos - FragPos);
    float diff    = max(dot(norm, lightDir), 0.0);
    vec3 diffuse  = diff * lightColor;

    vec3 result = (ambient + diffuse) * objectColor;

    // Sanity color shift: low sanity pushes toward desaturated purple-grey
    float insanity = 1.0 - sanity;
    result = mix(result, vec3(dot(result, vec3(0.299, 0.587, 0.114))) * vec3(0.6, 0.5, 0.7), insanity * 0.5);

    FragColor = vec4(result, 1.0);
}
