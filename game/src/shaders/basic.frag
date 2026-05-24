#version 330 core

in vec3 FragPos;
in vec3 Normal;
in vec2 TexCoord;

uniform vec3  lightPos;
uniform vec3  eyePos;
uniform vec3  lightColor;
uniform vec3  objectColor;
uniform float ambientStrength;
uniform float sanity;

out vec4 FragColor;

vec3 pointLight(vec3 lPos, vec3 lColor, float strength) {
    vec3  norm = normalize(Normal);
    vec3  dir  = normalize(lPos - FragPos);
    float diff = max(dot(norm, dir), 0.0);
    float dist = length(lPos - FragPos);
    float atten = 1.0 / (1.0 + 0.14 * dist + 0.07 * dist * dist);
    return diff * lColor * strength * atten;
}

void main() {
    // Warm ambient fill — stylized hotels are readable, not caves
    vec3 ambient = ambientStrength * lightColor;

    // Warm incandescent ceiling lamp
    vec3 overhead = pointLight(lightPos, vec3(1.0, 0.90, 0.72), 5.5);

    // Player-local fill light — soft warm glow around the player
    vec3 nearby = pointLight(eyePos, vec3(1.0, 0.96, 0.88), 2.8);

    vec3 result = (ambient + overhead + nearby) * objectColor;

    // Slight toon quantization — groups shading into 4 bands for stylized look
    float brightness = length(result);
    float band = floor(brightness * 4.0) / 4.0;
    result = result * (band / max(brightness, 0.001));
    result = clamp(result, 0.0, 1.0);

    // Sanity: at low sanity colours drain to cold sickly green-grey, then near-black
    float insanity = 1.0 - sanity;
    float lum = dot(result, vec3(0.299, 0.587, 0.114));
    vec3 cold = vec3(lum) * vec3(0.52, 0.60, 0.55); // sickly cold tint
    result = mix(result, cold, insanity * 0.85);
    result *= mix(1.0, 0.35, insanity * insanity); // darken dramatically at 0 sanity

    FragColor = vec4(result, 1.0);
}
