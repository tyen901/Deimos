cbuffer scope_view : register(b12)
{
    float4x4 world_to_projective : packoffset(c0);
    float4x4 camera_to_world : packoffset(c4);

    float4 target : packoffset(c8);
    float4 view_miscellaneous : packoffset(c9);
    float4 view_unk20 : packoffset(c10);
    float4x4 camera_to_projective : packoffset(c11);
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

float4 mainPS(VSOutput input)
    : SV_TARGET
{
    float4 rt0 = gbuffer_albedo.Sample(samplerState, input.uv);
    float4 rt1 = gbuffer_normal.Sample(samplerState, input.uv);
    float4 rt2 = gbuffer_third.Sample(samplerState, input.uv);
    float depth = deferred_depth.Sample(samplerState, input.uv).x;
    if (depth == 0)
        return float4(0.58, 0.78, 1, 1);

    // TODO after break: lil shading on the result, no biggie
    float3 albedo = rt0.rgb;
    float3 normal = rt1.xyz * 2.0 - 1.0;
    // float smoothness = length(normal) * 4 - 3;
    // normal = normalize(normal);
    // float metalness = rt2.r;
    float textureEmissive = saturate(rt2.g * 2.0 - 1.0);
    float textureAo = saturate(rt2.g * 2.0);
    // float transmission = saturate(rt2.g);
    // float vertexAo = rt2.a;
    // return float4(linearToSrgb(vertexAo.xxx), 1.0f);

    // // float aoFactor = saturate(textureAo * vertexAo);
    // float aoFactor = saturate(textureAo);
    float3 finalColor = albedo.rgb;

    // Simple Phong shading
    // float3 lightDir = -normalize(float3(-0.618537, 0.102473, -0.779045));
    float3 lightDir = -normalize(float3(-0.579228, 0.40558, -0.707107));
    float3 ambient = float(0.15).xxx;
    float3 diffuse = max(dot(normal, lightDir), 0.0) * (1 - ambient);

    // // Final color
    float3 light = (ambient + diffuse);
    // float4 shaded = shading_result.Sample(samplerState, input.uv);
    // return shaded;
    // float3 finalColor = shaded.rgb * light;
    // return float4(finalColor, 1.0f);

    // float3 light = light_diffuse.Sample(samplerState, input.uv).rgb;
    // return float4(linearToSrgb(light), 1.0f);
    light = max(textureEmissive.xxx, light);
    return float4(linearToSrgb(finalColor) * light, 1.0f);
}
