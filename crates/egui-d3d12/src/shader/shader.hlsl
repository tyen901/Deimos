struct vs_in {
  float2 position : POSITION;
  float2 uv : TEXCOORD;
  float4 color : COLOR;
};

struct vs_out {
  float4 clip : SV_POSITION;
  float2 uv : TEXCOORD;
  float4 color : COLOR;
};

vs_out vs_main(vs_in input) {
  vs_out output;
  output.clip = float4(input.position, 0.0, 1.0);
  output.uv = input.uv;
  output.color = input.color;

  return output;
}

float LinearToSRGB(float x) {
    return x < 0.0031308 ? 12.92 * x : 1.055 * pow(x, 1.0/2.4) - 0.055;
}

sampler sampler0;
Texture2D texture0;

float4 ps_main(vs_out input) : SV_TARGET {
  float4 t = texture0.Sample(sampler0, input.uv);
#ifdef CLEAR_ALPHA
  t.r = LinearToSRGB(t.r);
  t.g = LinearToSRGB(t.g);
  t.b = LinearToSRGB(t.b);
  t.a = 1.0;
#endif
  return float4(pow(input.color.rgb, 1.0 / 1.9) * t.rgb, input.color.a * t.a);
}
