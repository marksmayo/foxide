//!HOOK MAIN
//!BIND HOOKED
vec4 hook() { vec4 color = HOOKED_tex(HOOKED_pos); return vec4(vec3(1.0) - color.rgb, color.a); }
