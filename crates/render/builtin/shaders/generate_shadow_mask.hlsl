#include "include/screen_space_vs.hlsl"
#include "include/view.hlsl"

static const uint SAMPLE_NUM = 9;
static const float2 POISSON_SAMPLES[SAMPLE_NUM] = {
  float2(0.0f, 0.0f),
  float2(-0.45570817558838717f, -0.8078111571344746f),
  float2(0.8960089702138108f, -0.3569415776898711f),
  float2(-0.19807511750350795f, 0.9210419317460738f),
  float2(0.2595217206107728f, -0.8090512416351279f),
  float2(0.5972218724881266f, 0.7789149385216658f),
  float2(-0.8088925785649114f, 0.45309126356924434f),
  float2(-0.9167903846266768f, -0.21303266118076764f),
  float2(0.6772830402709191f, 0.18318818607984763f),
};

#define NUM_CASCADES 4
cbuffer scope_shadowmap : register(b0) {
  float4x4 world_to_cascade_projective[NUM_CASCADES];
};

// Textures
Texture2D<float> deferred_depth : register(t0);

Texture2D<float> cascade_0 : register(t10);
Texture2D<float> cascade_1 : register(t11);
Texture2D<float> cascade_2 : register(t12);
Texture2D<float> cascade_3 : register(t13);

SamplerState samplerState : register(s0);

float InterleavedGradientNoise(float2 position_screen) {
  float3 magic = float3(0.06711056f, 0.00583715f, 52.9829189f);
  return frac(magic.z * frac(dot(position_screen, magic.xy)));
}

float2 GetTexelSize(Texture2D<float> tex) {
  uint width, height;
  tex.GetDimensions(width, height);
  return float2(1.0f / width, 1.0f / height);
}

Texture2D<float> GetCascadeTex(uint cascade) {
  switch (cascade) {
  case 0:
    return cascade_0;
  case 1:
    return cascade_1;
  case 2:
    return cascade_2;
  case 3:
    return cascade_3;
  default:
    return cascade_0;
  }
}

float SampleShadow(float3 world_pos, float2 screen_pos, uint cascade) {
  float4 posLightSpace =
      mul(world_to_cascade_projective[cascade], float4(world_pos.xyz, 1.0));
  posLightSpace /= posLightSpace.w; // Normalize homogeneous coordinates

  float2 cascadeUv;
  cascadeUv.x = posLightSpace.x * 0.5 + 0.5;
  cascadeUv.y = 1.0 - (posLightSpace.y * 0.5 + 0.5);

  if (cascadeUv.x < 0.0 || cascadeUv.x > 1.0 || cascadeUv.y < 0.0 ||
      cascadeUv.y > 1.0)
    return 1.0;

  if (posLightSpace.z < 0.0 || posLightSpace.z > 1.0)
    return 1.0;

  float bias = 0.0001;
  Texture2D<float> shadowmap = GetCascadeTex(cascade);
  float2 texelSize = GetTexelSize(shadowmap);

  float filterSpread = 2.5;

  float randomAngle =
      InterleavedGradientNoise(screen_pos) * 6.28318530718; // 2 * PI
  float s = sin(randomAngle);
  float c = cos(randomAngle);

  float2x2 rot = float2x2(c, -s, s, c);

  float shadow = 0.0;

  [unroll] // Optional: unrolling can help performance for small loops
  for (int i = 0; i < SAMPLE_NUM; ++i) {
    float2 rotatedOffset = mul(rot, POISSON_SAMPLES[i]);
    float2 offset = rotatedOffset * texelSize * filterSpread;

    float shadowDepth = shadowmap.Sample(samplerState, cascadeUv + offset).x;
    shadow += (posLightSpace.z - bias) > shadowDepth ? 0.0 : 1.0;
  }
  shadow /= float(SAMPLE_NUM);

  return shadow;
}

float4 mainPS(VSOutput input) : SV_TARGET {

  float depth = deferred_depth.Sample(samplerState, input.uv);
  if (depth == 0.0f) {
    discard;
  }

  float4 cameraPos =
      mul(target_pixel_to_camera, float4(input.pos.xy, depth, 1.0f));
  cameraPos /= cameraPos.w;
  float3 worldPos = mul(camera_to_world, cameraPos).xyz;

  float shadow = 1.0;
  [unroll]
  for (int i = 0; i < NUM_CASCADES; i++) {
    shadow *= SampleShadow(worldPos, input.pos.xy, i);
  }

  return shadow.xxxx;
}
