#include "include/screen_space_vs.hlsl"
#include "include/view.hlsl"

// Textures
Texture2D<float4> gbuffer_normal : register(t0);
Texture2D<float4> gbuffer_third : register(t1);
Texture2D<float> deferred_depth : register(t2);
Texture2D<float4> hdri : register(t3);
SamplerState samplerState : register(s0);

static const float PI = 3.14159265359f;
static const float MAX_MIP = 10.0f;

float2 DirToEquirect(float3 dir) {
  float phi = atan2(dir.y, dir.x);
  float theta = asin(clamp(dir.z, -1.0f, 1.0f));
  return float2(phi / (2.0f * PI) + 0.5f, 0.5f - theta / PI);
}

float3 SampleHDRI(float3 dir, float mip) {
  return hdri.SampleLevel(samplerState, DirToEquirect(dir), mip).rgb * 1.5f;
}

float2 EnvBRDFApprox(float NoV, float roughness) {
  const float4 c0 = float4(-1.0f, -0.0275f, -0.572f, 0.022f);
  const float4 c1 = float4(1.0f, 0.0425f, 1.040f, -0.040f);
  float4 r = roughness * c0 + c1;
  float a004 = min(r.x * r.x, exp2(-9.28f * NoV)) * r.x + r.y;
  return saturate(float2(-1.04f, 1.04f) * a004 + r.zw);
}

float3 F_SchlickRoughness(float cosTheta, float3 F0, float roughness) {
  float3 hi =
      max(float3(1.0f - roughness, 1.0f - roughness, 1.0f - roughness), F0);
  return F0 + (hi - F0) * pow(saturate(1.0f - cosTheta), 5.0f);
}

float remap(float value, float min, float max) {
  return saturate((value - min) / (max - min));
}

void mainPS(VSOutput input, out float4 light_diffuse: SV_TARGET0,
            out float4 light_specular: SV_TARGET1) {

  float depth = deferred_depth.Sample(samplerState, input.uv);
  if (depth == 0.0f) {
    discard;
  }

  float4 cameraPos =
      mul(target_pixel_to_camera, float4(input.pos.xy, depth, 1.0f));
  cameraPos /= cameraPos.w;
  float3 worldPos = mul(camera_to_world, cameraPos).xyz;

  float4 rt1 = gbuffer_normal.Sample(samplerState, input.uv);
  float4 rt2 = gbuffer_third.Sample(samplerState, input.uv);

  float3 rawNormal = rt1.xyz * 2.0f - 1.0f;
  float3 N = normalize(rawNormal);

  float smoothness = saturate(length(rawNormal) * 4.0f - 3.0f);
  float roughness = 1.0f - smoothness;

  float metallic = rt2.r;
  float textureEmissive = saturate(rt2.g * 2.0f - 1.0f);
  float textureAo = saturate(rt2.g * 2.0f);

  float3 V = normalize(camera_position - worldPos);
  float3 R = reflect(-V, N);

  float NoV = max(dot(N, V), 1e-4f);

  float3 F0 =
      lerp(float3(0.04f, 0.04f, 0.04f), float3(1.0f, 1.0f, 1.0f), metallic);

  float3 irradiance = SampleHDRI(N, MAX_MIP);

  float3 kS = F_SchlickRoughness(NoV, F0, roughness);
  float3 kD = (1.0f - kS) * (1.0f - metallic);

  float3 diffuse = kD * irradiance * textureAo;

  float specularMip = roughness * MAX_MIP;
  float3 LD = SampleHDRI(R, specularMip);
  float2 dfg = EnvBRDFApprox(NoV, roughness);

  float3 specular = LD * (F0 * dfg.x + dfg.y) * textureAo;

  light_diffuse = float4(diffuse, 1.0f);
  light_specular = float4(specular, 1.0f);
}
