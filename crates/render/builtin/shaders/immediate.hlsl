#include "include/view.hlsl"

struct VSInput
{
    float3 pos : POSITION;
    float4 color : COLOR;
};

struct VSOutput
{
    float4 pos : SV_POSITION;
    float4 color : TEXCOORD0;
};

VSOutput mainVS(VSInput input)
{
    VSOutput output;

    output.pos = mul(world_to_projective, float4(input.pos, 1.0f));
    output.color = input.color;

    return output;
}

float4 mainPS(VSOutput input) : SV_Target
{
    return input.color;
}
