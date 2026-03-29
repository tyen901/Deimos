dxc -Fo apply_hdri_as_background.ps.dxil apply_hdri_as_background.hlsl -T ps_6_0 -E mainPS
dxc -Fo apply_hdri_as_background.vs.dxil apply_hdri_as_background.hlsl -T vs_6_0 -E mainVS
dxc -Fo apply_hdri_as_light.ps.dxil apply_hdri_as_light.hlsl -T ps_6_0 -E mainPS
dxc -Fo apply_hdri_as_light.vs.dxil apply_hdri_as_light.hlsl -T vs_6_0 -E mainVS
dxc -Fo immediate.ps.dxil immediate.hlsl -T ps_6_0 -E mainPS
dxc -Fo immediate.vs.dxil immediate.hlsl -T vs_6_0 -E mainVS
