cbuffer scope_view : register(b12)
{
    float4x4 world_to_projective : packoffset(c0);
    float4x4 camera_to_world : packoffset(c4);

    float4x4 target_pixel_to_camera : packoffset(c8);
    float4 target : packoffset(c12);
    float4 view_miscellaneous : packoffset(c13);
    // float4 target : packoffset(c8);
    // float4 view_miscellaneous : packoffset(c9);
    // float4 view_unk20 : packoffset(c10);
    // float4x4 camera_to_projective : packoffset(c11);
}; // cbuffer scope_view

#define camera_position (transpose(camera_to_world)[3].xyz)
#define camera_backward (transpose(camera_to_world)[2].xyz)
#define camera_up (transpose(camera_to_world)[1].xyz)
#define camera_right (transpose(camera_to_world)[0].xyz)
#define camera_forward (-transpose(camera_to_world)[2].xyz)
#define camera_down (-transpose(camera_to_world)[1].xyz)
#define camera_left (-transpose(camera_to_world)[0].xyz)
#define target_width (target.x)
#define target_height (target.y)
#define target_resolution (target.xy)
#define inverse_target_resolution (target.zw)
#define maximum_depth_pre_projection (view_miscellaneous.x)
#define view_is_first_person (view_miscellaneous.y)

// Vertex Shader
struct VSOutput
{
    float4 pos : SV_POSITION;
    float2 uv : TEXCOORD0;
};

static const float4 vertices[4] = {
    float4(-1.0, 1.0, 0.0, 1.0),  // Top Left
    float4(1.0, 1.0, 0.0, 1.0),   // Top Right
    float4(-1.0, -1.0, 0.0, 1.0), // Bottom Left
    float4(1.0, -1.0, 0.0, 1.0)   // Bottom Right
};

static const float2 uvs[4] = {
    float2(0.0, 0.0), // Top Left
    float2(1.0, 0.0), // Top Right
    float2(0.0, 1.0), // Bottom Left
    float2(1.0, 1.0)  // Bottom Right
};

VSOutput mainVS(uint vertexID: SV_VertexID)
{
    VSOutput output;

    output.pos = vertices[vertexID];
    output.uv = uvs[vertexID];

    return output;
}

// Pixel Shader
Texture2D gbuffer_albedo : register(t0);
Texture2D gbuffer_normal : register(t1);
Texture2D gbuffer_third : register(t2);
Texture2D deferred_depth : register(t3);
Texture2D shading_result : register(t4);
SamplerState samplerState : register(s0);

float3 linearToSrgb(float3 r0)
{
    r0 = log2(r0.xyz);
    r0 = float3(0.454545468, 0.454545468, 0.454545468) * r0.xyz;
    return exp2(r0.xyz);
}

// Remaps the given value from the range [min, max] to [0, 1].
float remap(float value, float min, float max)
{
    return saturate((value - min) / (max - min));
}

static float3 SkyColor = float3(0.58, 0.78, 1);

float4 mainPS(VSOutput input)
    : SV_TARGET
{
    float4 rt0 = gbuffer_albedo.Sample(samplerState, input.uv);
    float4 rt1 = gbuffer_normal.Sample(samplerState, input.uv);
    float4 rt2 = gbuffer_third.Sample(samplerState, input.uv);
    float depth = deferred_depth.Sample(samplerState, input.uv).x;
    if (depth == 0)
        return float4(SkyColor, 1);

    float4 worldPos = mul(target_pixel_to_camera, float4(input.pos.xy, depth, 1));
    worldPos /= worldPos.w;
    worldPos = mul(camera_to_world, worldPos);
    worldPos /= worldPos.w;

    float distance = length(worldPos.xyz - camera_position);
    float fog = remap(distance, 275.0, 350.0);

    // TODO after break: specular shading since we got worldpos now
    float3 albedo = rt0.rgb;
    float3 normal = rt1.xyz * 2.0 - 1.0;
    float smoothness = saturate(length(normal) * 4 - 3);
    normal = normalize(normal);
    // float metalness = rt2.r;
    float textureEmissive = saturate(rt2.g * 2.0 - 1.0);
    float textureAo = saturate(rt2.g * 2.0);
    // float transmission = saturate(rt2.g);
    // float vertexAo = rt2.a;
    // return float4(linearToSrgb(vertexAo.xxx), 1.0f);

    // // float aoFactor = saturate(textureAo * vertexAo);
    // float aoFactor = saturate(textureAo);
    float3 finalColor = albedo.rgb;

    // Blinn-Phong shading
    float3 lightDir = -normalize(float3(-0.579228, 0.40558, -0.707107));
    // float3 lightDir = normalize(lightPos - FragPos);
    float3 viewDir = normalize(camera_position - worldPos.xyz);
    float3 halfwayDir = normalize(lightDir + viewDir);

    float3 ambient = float(0.15).xxx;
    float3 diffuse = max(dot(normal, lightDir), 0.0) * (1 - ambient);
    float spec = pow(max(dot(normal, halfwayDir), 0.0), 32.0) * smoothness;

    // Final color
    float3 light = (ambient + diffuse) + spec.xxx;

    // float3 light = light_diffuse.Sample(samplerState, input.uv).rgb;
    // return float4(linearToSrgb(light), 1.0f);
    light = max(textureEmissive.xxx, light);
    float3 c = linearToSrgb(finalColor) * light;
    return float4(lerp(c, SkyColor, fog), 1.0f);
}
