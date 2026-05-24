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
    float atten = 1.0 / (1.0 + 0.18 * dist + 0.06 * dist * dist);
    return diff * lColor * strength * atten;
}

// Boost saturation to make colours feel rich and warm
vec3 saturate(vec3 col, float amount) {
    float lum = dot(col, vec3(0.299, 0.587, 0.114));
    return mix(vec3(lum), col, amount);
}

void main() {
    // Warm ambient — hotel should feel inhabited, not like a cave
    vec3 ambient = ambientStrength * vec3(1.0, 0.96, 0.88);

    // Soft warm ceiling lamp
    vec3 overhead = pointLight(lightPos, vec3(1.0, 0.92, 0.76), 2.2);

    // Player fill — just enough to stop corners going pitch black
    vec3 nearby = pointLight(eyePos, vec3(1.0, 0.97, 0.90), 1.0);

    vec3 lit = (ambient + overhead + nearby) * objectColor;

    // Slight saturation boost for the stylized "warm hotel" feel
    lit = saturate(lit, 1.25);
    lit = clamp(lit, 0.0, 1.0);

    // ── Sanity horror effect ──────────────────────────────────────────────
    float insanity = 1.0 - sanity;

    // Phase 1 (insanity 0–0.5): very gentle warmth drain, barely noticeable
    // Phase 2 (insanity 0.5–1.0): dramatic cold shift + darkening
    float phase2 = clamp((insanity - 0.5) / 0.5, 0.0, 1.0);

    float lum = dot(lit, vec3(0.299, 0.587, 0.114));
    vec3 cold  = vec3(lum) * vec3(0.55, 0.62, 0.58); // sickly grey-green
    lit = mix(lit, cold, phase2 * 0.90);

    // Darken toward near-black only in phase 2
    lit *= mix(1.0, 0.28, phase2 * phase2);

    FragColor = vec4(lit, 1.0);
}
