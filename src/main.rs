//! Animated mathematical wallpaper for Sway/Wayland.
//!
//! GPU-accelerated rendering using EGL + OpenGL ES 2.0 on top of
//! smithay-client-toolkit with wlr-layer-shell. The GPU draws anti-aliased
//! curve segments into a framebuffer object (FBO) that accumulates over time.
//! Each frame the CPU only computes 3 pendulum positions and submits 3
//! triangle-strip draw calls — all rasterization happens on the GPU.
//!
//! When no Wayland display is available (or `--ascii` is passed), falls back
//! to a terminal ASCII renderer using the same math and color palette.

mod control;

use std::env;
use std::io::Write;
use std::time::Duration;

use calloop::timer::{TimeoutAction, Timer};
use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;
use control::ControlSocket;
use glow::HasContext;
use log::{info, warn};
use wl_walls::shapes::{self, CurveDrawer};
use wl_walls::{Color, colors_from_env, parse_env_f32, parse_env_f64, parse_env_u32, parse_hex_color, pick_random_color, resolve_shape_env};
use rand::Rng;
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    registry_handlers,
};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_output, wl_surface};
use wayland_client::{Connection, Proxy, QueueHandle};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Scale factor applied to the smaller display dimension for aspect-ratio normalization.
const ASPECT_SCALE: f64 = 0.8;

/// Divisor for converting line width from user-facing pixels to NDC units.
const LINE_WIDTH_DIVISOR: f64 = 6.0;

/// Number of Catmull-Rom subdivisions per curve segment.
const CURVE_SUBDIVISIONS: usize = 16;

// ---------------------------------------------------------------------------
// GL renderer
// ---------------------------------------------------------------------------

struct GlRenderer {
    gl: glow::Context,
    program: glow::Program,
    vbo: glow::Buffer,
    fbo: glow::Framebuffer,
    fbo_texture: glow::Texture,
    blit_program: glow::Program,
    blit_vbo: glow::Buffer,
    u_color: glow::UniformLocation,
    u_bg: glow::UniformLocation,
    a_pos: u32,
    a_cross: u32,
    width: u32,
    height: u32,
    u_dither_strength: glow::UniformLocation,
    u_dither_levels: glow::UniformLocation,
    u_dither_scale: glow::UniformLocation,
    u_resolution: glow::UniformLocation,
}

impl GlRenderer {
    unsafe fn new(gl: glow::Context, width: u32, height: u32) -> Self {
        // --- Line drawing shader with edge antialiasing ---
        let vs_src = r#"#version 100
            attribute vec2 a_pos;
            attribute float a_cross;
            varying float v_cross;
            void main() {
                v_cross = a_cross;
                gl_Position = vec4(a_pos, 0.0, 1.0);
            }
        "#;
        let fs_src = r#"#version 100
            precision mediump float;
            uniform vec4 u_color;
            varying float v_cross;
            void main() {
                float d = abs(v_cross);
                // Gaussian falloff: solid core in center, soft edges.
                // exp(-8 * d^2) gives ~full opacity up to d≈0.3, then
                // a smooth bell-curve fade to near-zero at the edges.
                float alpha = exp(-8.0 * d * d);
                gl_FragColor = vec4(u_color.rgb, u_color.a * alpha);
            }
        "#;
        let program = Self::create_program(&gl, vs_src, fs_src);
        let u_color = gl.get_uniform_location(program, "u_color").expect("uniform u_color");
        let a_pos = gl.get_attrib_location(program, "a_pos").expect("attrib a_pos");
        let a_cross = gl.get_attrib_location(program, "a_cross").expect("attrib a_cross");

        // --- Blit shader (composite FBO over background color) ---
        let blit_vs = r#"#version 100
            attribute vec2 a_pos;
            varying vec2 v_uv;
            void main() {
                v_uv = a_pos * 0.5 + 0.5;
                gl_Position = vec4(a_pos, 0.0, 1.0);
            }
        "#;
        let blit_fs = r#"#version 100
            precision mediump float;
            varying vec2 v_uv;
            uniform sampler2D u_tex;
            uniform vec3 u_bg;
            uniform float u_dither_strength;
            uniform float u_dither_levels;
            uniform float u_dither_scale;
            uniform vec2 u_resolution;

            // 8x8 ordered (Bayer) dithering threshold via recursive quadrant
            // decomposition. Returns a value in [0, 1).
            float bayer8(vec2 coord) {
                vec2 p = mod(floor(coord), 8.0);
                float value = 0.0;
                float size = 4.0;
                for (int i = 0; i < 3; i++) {
                    vec2 h = step(size, p);
                    p = mod(p, size);
                    value = value * 4.0 + 2.0 * h.x + 3.0 * h.y - 4.0 * h.x * h.y;
                    size *= 0.5;
                }
                return value / 64.0;
            }

            void main() {
                vec4 texel = texture2D(u_tex, v_uv);
                vec3 color = mix(u_bg, texel.rgb, texel.a);

                if (u_dither_strength > 0.0) {
                    vec2 fragCoord = v_uv * u_resolution / u_dither_scale;
                    float threshold = bayer8(fragCoord);
                    float levels = max(u_dither_levels, 2.0);
                    vec3 quantized = floor(color * (levels - 1.0) + threshold) / (levels - 1.0);
                    color = mix(color, clamp(quantized, 0.0, 1.0), u_dither_strength);
                }

                gl_FragColor = vec4(color, 1.0);
            }
        "#;
        let blit_program = Self::create_program(&gl, blit_vs, blit_fs);
        let u_bg = gl.get_uniform_location(blit_program, "u_bg").expect("uniform u_bg");
        let u_dither_strength = gl.get_uniform_location(blit_program, "u_dither_strength").expect("uniform u_dither_strength");
        let u_dither_levels = gl.get_uniform_location(blit_program, "u_dither_levels").expect("uniform u_dither_levels");
        let u_dither_scale = gl.get_uniform_location(blit_program, "u_dither_scale").expect("uniform u_dither_scale");
        let u_resolution = gl.get_uniform_location(blit_program, "u_resolution").expect("uniform u_resolution");

        // VBO for line strips
        let vbo = gl.create_buffer().expect("create VBO");

        // Fullscreen quad VBO for blit
        let blit_vbo = gl.create_buffer().expect("create blit VBO");
        #[rustfmt::skip]
        let quad: [f32; 12] = [
            -1.0, -1.0,  1.0, -1.0, -1.0,  1.0,
            -1.0,  1.0,  1.0, -1.0,  1.0,  1.0,
        ];
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(blit_vbo));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, cast_f32_slice(&quad), glow::STATIC_DRAW);

        // Create FBO + texture
        let (fbo, fbo_texture) = Self::create_fbo(&gl, width, height);

        // Clear FBO to fully transparent
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.viewport(0, 0, width as i32, height as i32);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);

        Self {
            gl,
            program,
            vbo,
            fbo,
            fbo_texture,
            blit_program,
            blit_vbo,
            u_color,
            u_bg,
            a_pos,
            a_cross,
            width,
            height,
            u_dither_strength,
            u_dither_levels,
            u_dither_scale,
            u_resolution,
        }
    }

    unsafe fn create_program(gl: &glow::Context, vs_src: &str, fs_src: &str) -> glow::Program {
        let program = gl.create_program().expect("create program");
        let vs = gl.create_shader(glow::VERTEX_SHADER).expect("create vertex shader");
        gl.shader_source(vs, vs_src);
        gl.compile_shader(vs);
        assert!(
            gl.get_shader_compile_status(vs),
            "VS: {}",
            gl.get_shader_info_log(vs)
        );
        let fs = gl.create_shader(glow::FRAGMENT_SHADER).expect("create fragment shader");
        gl.shader_source(fs, fs_src);
        gl.compile_shader(fs);
        assert!(
            gl.get_shader_compile_status(fs),
            "FS: {}",
            gl.get_shader_info_log(fs)
        );
        gl.attach_shader(program, vs);
        gl.attach_shader(program, fs);
        gl.link_program(program);
        assert!(
            gl.get_program_link_status(program),
            "Link: {}",
            gl.get_program_info_log(program)
        );
        gl.delete_shader(vs);
        gl.delete_shader(fs);
        program
    }

    unsafe fn create_fbo(
        gl: &glow::Context,
        width: u32,
        height: u32,
    ) -> (glow::Framebuffer, glow::Texture) {
        let tex = gl.create_texture().expect("create FBO texture");
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32,
            width as i32,
            height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(None),
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
        );

        let fbo = gl.create_framebuffer().expect("create FBO");
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
        gl.framebuffer_texture_2d(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::TEXTURE_2D,
            Some(tex),
            0,
        );
        assert_eq!(
            gl.check_framebuffer_status(glow::FRAMEBUFFER),
            glow::FRAMEBUFFER_COMPLETE
        );
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        (fbo, tex)
    }

    /// Reduce the alpha of every pixel in the FBO, keeping RGB intact.
    /// This makes older lines become more transparent each frame while
    /// preserving their original color/saturation.
    unsafe fn fade(&self, fade_amount: f32) {
        let gl = &self.gl;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.viewport(0, 0, self.width as i32, self.height as i32);

        gl.enable(glow::BLEND);
        // Color: keep unchanged (dst * 1)
        // Alpha: multiply by (1 - fade_amount) via dst * src_alpha
        gl.blend_func_separate(
            glow::ZERO,
            glow::ONE,
            glow::ZERO,
            glow::ONE_MINUS_SRC_ALPHA,
        );

        gl.use_program(Some(self.program));
        gl.uniform_4_f32(Some(&self.u_color), 0.0, 0.0, 0.0, fade_amount);

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.blit_vbo));
        gl.enable_vertex_attrib_array(self.a_pos);
        gl.vertex_attrib_pointer_f32(self.a_pos, 2, glow::FLOAT, false, 8, 0);
        gl.disable_vertex_attrib_array(self.a_cross);
        gl.vertex_attrib_1_f32(self.a_cross, 0.0);

        gl.draw_arrays(glow::TRIANGLES, 0, 6);

        gl.disable_vertex_attrib_array(self.a_pos);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }

    /// Draw a triangle strip (the thickened curve segment) into the FBO.
    /// Vertices are packed as [x, y, cross] where cross is -1.0 or +1.0
    /// indicating which side of the line center the vertex is on (for AA).
    unsafe fn draw_strip(&self, vertices: &[[f32; 3]], color: Color, alpha: f32) {
        let gl = &self.gl;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.viewport(0, 0, self.width as i32, self.height as i32);

        gl.enable(glow::BLEND);
        // Porter-Duff "over": proper alpha compositing into the FBO
        gl.blend_func_separate(
            glow::SRC_ALPHA,
            glow::ONE_MINUS_SRC_ALPHA,
            glow::ONE,
            glow::ONE_MINUS_SRC_ALPHA,
        );

        gl.use_program(Some(self.program));
        gl.uniform_4_f32(
            Some(&self.u_color),
            color.0 as f32,
            color.1 as f32,
            color.2 as f32,
            alpha,
        );

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
        gl.buffer_data_u8_slice(
            glow::ARRAY_BUFFER,
            cast_vert_slice(vertices),
            glow::STREAM_DRAW,
        );

        // stride = 12 bytes (3 floats × 4 bytes)
        gl.enable_vertex_attrib_array(self.a_pos);
        gl.vertex_attrib_pointer_f32(self.a_pos, 2, glow::FLOAT, false, 12, 0);

        gl.enable_vertex_attrib_array(self.a_cross);
        gl.vertex_attrib_pointer_f32(self.a_cross, 1, glow::FLOAT, false, 12, 8);

        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, vertices.len() as i32);

        gl.disable_vertex_attrib_array(self.a_pos);
        gl.disable_vertex_attrib_array(self.a_cross);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }

    /// Blit the FBO texture to the default framebuffer, compositing over bg.
    unsafe fn blit_to_screen(
        &self,
        bg: Color,
        dither_strength: f32,
        dither_levels: f32,
        dither_scale: f32,
    ) {
        let gl = &self.gl;
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.viewport(0, 0, self.width as i32, self.height as i32);

        gl.disable(glow::BLEND);
        gl.use_program(Some(self.blit_program));
        gl.uniform_3_f32(Some(&self.u_bg), bg.0 as f32, bg.1 as f32, bg.2 as f32);
        gl.uniform_1_f32(Some(&self.u_dither_strength), dither_strength);
        gl.uniform_1_f32(Some(&self.u_dither_levels), dither_levels);
        gl.uniform_1_f32(Some(&self.u_dither_scale), dither_scale);
        gl.uniform_2_f32(Some(&self.u_resolution), self.width as f32, self.height as f32);

        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(self.fbo_texture));

        gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.blit_vbo));
        let a_pos = gl.get_attrib_location(self.blit_program, "a_pos").expect("attrib a_pos");
        gl.enable_vertex_attrib_array(a_pos);
        gl.vertex_attrib_pointer_f32(a_pos, 2, glow::FLOAT, false, 8, 0);

        gl.draw_arrays(glow::TRIANGLES, 0, 6);

        gl.disable_vertex_attrib_array(a_pos);
    }

    /// Clear the FBO to fully transparent.
    unsafe fn clear(&self) {
        let gl = &self.gl;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
        gl.viewport(0, 0, self.width as i32, self.height as i32);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
    }
}

/// Cast &[f32] → &[u8].
fn cast_f32_slice(data: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 4) }
}

/// Cast &[[f32; 3]] → &[u8].
fn cast_vert_slice(data: &[[f32; 3]]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * 12) }
}

// ---------------------------------------------------------------------------
// Per-output surface state
// ---------------------------------------------------------------------------

struct OutputSurface {
    layer: LayerSurface,
    width: u32,
    height: u32,
    configured: bool,
    egl_surface: khronos_egl::Surface,
    _wl_egl_surface: wayland_egl::WlEglSurface,
    renderer: Option<GlRenderer>,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct App {
    registry_state: RegistryState,
    output_state: OutputState,
    compositor_state: CompositorState,
    layer_shell: LayerShell,
    shm: Shm,
    qh: QueueHandle<Self>,

    egl: khronos_egl::Instance<khronos_egl::Static>,
    egl_display: khronos_egl::Display,
    egl_context: khronos_egl::Context,
    egl_config: khronos_egl::Config,

    control: Option<ControlSocket>,
    outputs: Vec<(wl_output::WlOutput, Option<OutputSurface>)>,
    curve: CurveDrawer,
    shape_lock: Option<String>,
    fg_colors: Vec<Color>,
    bg_color: Color,
    current_color: Color,
    steps_per_tick: u32,
    fade_amount: f32,
    line_width: f64,
    line_alpha: f32,
    dither_strength: f32,
    dither_levels: f32,
    dither_scale: f32,
    /// Per-axis NDC scale factors to keep the pattern square.
    /// Computed from the first configured output's dimensions.
    scale_x: f64,
    scale_y: f64,
}

impl App {
    /// Make each output's EGL surface current and clear its renderer.
    fn clear_all_outputs(&mut self) {
        for (_wl, osurface) in &mut self.outputs {
            if let Some(os) = osurface {
                if let Some(ref renderer) = os.renderer {
                    self.egl
                        .make_current(
                            self.egl_display,
                            Some(os.egl_surface),
                            Some(os.egl_surface),
                            Some(self.egl_context),
                        )
                        .expect("EGL make_current for clear");
                    unsafe { renderer.clear() };
                }
            }
        }
    }

    fn pick_new_color(&mut self) {
        self.current_color = pick_random_color(&self.fg_colors, self.current_color);
    }

    fn restart(&mut self) {
        self.clear_all_outputs();
        if let Some(ref lock) = self.shape_lock {
            // Locked to a specific shape — re-randomize same type
            let name = lock.clone();
            self.curve.switch_shape(&name);
        } else {
            self.curve.randomize_new_shape();
        }
        self.pick_new_color();
        info!("New pattern: shape={}, color=({:.2},{:.2},{:.2})",
              self.curve.shape.name(),
              self.current_color.0, self.current_color.1, self.current_color.2);
    }

    fn tick(&mut self) {
        self.poll_control();
        let color = self.current_color;
        let bg = self.bg_color;
        let steps = self.steps_per_tick;
        let mut advanced = false;

        for _ in 0..steps {
            if !self.curve.advance() {
                // Render whatever we have, then restart
                self.render_all_outputs(advanced, color, bg);
                self.restart();
                self.render_all_outputs(false, color, bg);
                return;
            }
            advanced = true;
        }

        if advanced {
            self.render_all_outputs(true, color, bg);
        }
    }

    /// Perform all GL work for one frame on every output.
    /// Computes per-output vertices so line width is consistent in pixels.
    fn render_all_outputs(&mut self, draw: bool, color: Color, bg: Color) {
        let mut verts: Vec<[f32; 3]> = Vec::new();

        for (_wl, osurface) in &mut self.outputs {
            if let Some(os) = osurface {
                if !os.configured {
                    continue;
                }
                if let Some(ref renderer) = os.renderer {
                    self.egl
                        .make_current(
                            self.egl_display,
                            Some(os.egl_surface),
                            Some(os.egl_surface),
                            Some(self.egl_context),
                        )
                        .expect("EGL make_current");
                    unsafe {
                        renderer.fade(self.fade_amount);
                        if draw {
                            let line_width =
                                self.line_width * LINE_WIDTH_DIVISOR / os.height.min(os.width) as f64;
                            verts.clear();
                            self.curve.append_catmull_rom_strip(
                                self.scale_x,
                                self.scale_y,
                                line_width,
                                CURVE_SUBDIVISIONS,
                                &mut verts,
                            );
                            if !verts.is_empty() {
                                renderer.draw_strip(&verts, color, self.line_alpha);
                            }
                        }
                        renderer.blit_to_screen(
                            bg,
                            self.dither_strength,
                            self.dither_levels,
                            self.dither_scale,
                        );
                    }
                    self.egl
                        .swap_buffers(self.egl_display, os.egl_surface)
                        .expect("EGL swap_buffers");
                    os.layer
                        .wl_surface()
                        .damage_buffer(0, 0, os.width as i32, os.height as i32);
                    os.layer.commit();
                }
            }
        }
    }

    fn create_surface_for_output(&mut self, wl_output: &wl_output::WlOutput) {
        let info = self.output_state.info(wl_output);
        let (width, height) = info
            .as_ref()
            .and_then(|i| i.logical_size)
            .map(|(w, h)| (w as u32, h as u32))
            .unwrap_or((1920, 1080));

        info!(
            "Creating layer surface for output: {:?} ({}x{})",
            info.as_ref().and_then(|i| i.name.as_deref()),
            width,
            height
        );

        let surface = self.compositor_state.create_surface(&self.qh);
        let layer = self.layer_shell.create_layer_surface(
            &self.qh,
            surface,
            Layer::Background,
            Some("wl-walls"),
            Some(wl_output),
        );
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.set_size(width, height);
        layer.commit();

        // Create EGL surface
        let wl_surface = layer.wl_surface();
        let wl_egl_surface =
            wayland_egl::WlEglSurface::new(wl_surface.id(), width as i32, height as i32)
                .expect("create WlEglSurface");

        let egl_surface = unsafe {
            self.egl
                .create_window_surface(
                    self.egl_display,
                    self.egl_config,
                    wl_egl_surface.ptr() as khronos_egl::NativeWindowType,
                    None,
                )
                .expect("create EGL surface")
        };

        // Disable vsync — we drive frame pacing with calloop timer
        self.egl
            .make_current(
                self.egl_display,
                Some(egl_surface),
                Some(egl_surface),
                Some(self.egl_context),
            )
            .expect("EGL make_current");
        self.egl.swap_interval(self.egl_display, 0).expect("EGL swap_interval");

        let os = OutputSurface {
            layer,
            width,
            height,
            configured: false,
            egl_surface,
            _wl_egl_surface: wl_egl_surface,
            renderer: None,
        };

        for (wl, slot) in &mut self.outputs {
            if wl == wl_output {
                *slot = Some(os);
                return;
            }
        }
        self.outputs.push((wl_output.clone(), Some(os)));
    }

    fn init_renderer_for_output(&mut self, idx: usize) {
        if let Some((_wl, Some(os))) = self.outputs.get_mut(idx) {
            if os.renderer.is_some() {
                return;
            }
            self.egl
                .make_current(
                    self.egl_display,
                    Some(os.egl_surface),
                    Some(os.egl_surface),
                    Some(self.egl_context),
                )
                .expect("EGL make_current");

            let gl = unsafe {
                glow::Context::from_loader_function(|name| {
                    // This runs only during init, not per-frame
                    let egl = khronos_egl::Instance::new(khronos_egl::Static);
                    egl.get_proc_address(name)
                        .map_or(std::ptr::null(), |p| p as *const _)
                })
            };

            let renderer = unsafe { GlRenderer::new(gl, os.width, os.height) };
            os.renderer = Some(renderer);
            info!("GL renderer initialized for {}x{}", os.width, os.height);
        }
    }

    /// Recompute NDC scale factors from the largest configured output.
    ///
    /// The Python version used `scale = min(w, h) * 0.4` in pixel coordinates
    /// for both axes, keeping the pattern square. In NDC [-1, 1], each axis
    /// spans its full pixel dimension, so we need:
    ///   scale_x = min(w, h) * 0.4 / (w / 2) = min(w, h) * 0.8 / w
    ///   scale_y = min(w, h) * 0.4 / (h / 2) = min(w, h) * 0.8 / h
    /// For the shorter axis this gives 0.8, for the longer it's smaller.
    fn update_scales(&mut self) {
        let mut max_w = 0u32;
        let mut max_h = 0u32;
        for (_wl, osurface) in &self.outputs {
            if let Some(os) = osurface {
                if os.configured {
                    max_w = max_w.max(os.width);
                    max_h = max_h.max(os.height);
                }
            }
        }
        if max_w > 0 && max_h > 0 {
            let min_dim = max_w.min(max_h) as f64;
            self.scale_x = min_dim * ASPECT_SCALE / max_w as f64;
            self.scale_y = min_dim * ASPECT_SCALE / max_h as f64;
            info!(
                "Updated scales: {:.3} x {:.3} (from {}x{})",
                self.scale_x, self.scale_y, max_w, max_h
            );
        }
    }

    // -----------------------------------------------------------------------
    // Control socket
    // -----------------------------------------------------------------------

    fn poll_control(&mut self) {
        let control = match self.control.take() {
            Some(c) => c,
            None => return,
        };
        let pending = control.collect_pending();
        self.control = Some(control);

        for (cmd, mut stream) in pending {
            let response = self.handle_command(&cmd);
            let _ = stream.write_all(response.as_bytes());
        }
    }

    fn handle_command(&mut self, cmd: &str) -> String {
        let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
        match parts.first().copied() {
            Some("get") => self.cmd_get(),
            Some("set") if parts.len() >= 3 => self.cmd_set(parts[1], parts[2]),
            Some("set") => "error: usage: set <param> <value>\n".into(),
            Some("restart") => self.cmd_restart(),
            Some("randomize") => self.cmd_randomize(),
            Some("next-color") => self.cmd_next_color(),
            Some("next-shape") => self.cmd_next_shape(),
            _ => format!("error: unknown command '{}'\n", cmd),
        }
    }

    fn cmd_get(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(out, "shape={}", self.curve.shape.name());
        let _ = writeln!(out, "line_width={}", self.line_width);
        let _ = writeln!(out, "alpha={}", self.line_alpha);
        let _ = writeln!(out, "fade={}", self.fade_amount);
        let _ = writeln!(out, "speed={}", self.steps_per_tick);
        let _ = writeln!(out, "dither={}", self.dither_strength);
        let _ = writeln!(out, "dither_levels={}", self.dither_levels);
        let _ = writeln!(out, "dither_scale={}", self.dither_scale);
        let _ = writeln!(out, "bg={},{},{}", self.bg_color.0, self.bg_color.1, self.bg_color.2);
        let _ = writeln!(out, "color={},{},{}", self.current_color.0, self.current_color.1, self.current_color.2);
        for (name, val) in self.curve.shape.all_params() {
            let _ = writeln!(out, "{}={}", name, val);
        }
        out
    }

    fn cmd_set(&mut self, param: &str, value_str: &str) -> String {
        macro_rules! parse_set {
            ($field:expr, $ty:ty, $min:expr, $max:expr) => {
                match value_str.parse::<$ty>() {
                    Ok(v) => {
                        $field = v.clamp($min, $max);
                        return "ok\n".into();
                    }
                    Err(_) => return "error: invalid value\n".into(),
                }
            };
        }

        match param {
            "line_width" => parse_set!(self.line_width, f64, 0.5, 50.0),
            "alpha" => parse_set!(self.line_alpha, f32, 0.01, 1.0),
            "fade" => parse_set!(self.fade_amount, f32, 0.0, 1.0),
            "speed" => parse_set!(self.steps_per_tick, u32, 1, 100),
            "dither" => parse_set!(self.dither_strength, f32, 0.0, 1.0),
            "dither_levels" => parse_set!(self.dither_levels, f32, 2.0, 256.0),
            "dither_scale" => parse_set!(self.dither_scale, f32, 1.0, 16.0),
            "bg" => {
                if let Some(c) = parse_hex_color(value_str) {
                    self.bg_color = c;
                    "ok\n".into()
                } else {
                    "error: invalid hex color\n".into()
                }
            }
            "shape" => {
                if shapes::SHAPE_NAMES.contains(&value_str) {
                    self.curve.switch_shape(value_str);
                    // Clear all outputs for the new shape
                    self.clear_all_outputs();
                    "ok\n".into()
                } else {
                    format!(
                        "error: unknown shape '{}' (available: {})\n",
                        value_str,
                        shapes::SHAPE_NAMES.join(", ")
                    )
                }
            }
            _ => match value_str.parse::<f64>() {
                Ok(v) => {
                    if self.curve.shape.set_param(param, v) {
                        "ok\n".into()
                    } else {
                        format!("error: unknown param '{}'\n", param)
                    }
                }
                Err(_) => "error: invalid value\n".into(),
            },
        }
    }

    fn cmd_restart(&mut self) -> String {
        self.curve.reset_time();
        self.clear_all_outputs();
        "ok\n".into()
    }

    fn cmd_randomize(&mut self) -> String {
        self.restart();
        "ok\n".into()
    }

    fn cmd_next_color(&mut self) -> String {
        self.pick_new_color();
        "ok\n".into()
    }

    fn cmd_next_shape(&mut self) -> String {
        let next = shapes::next_shape_name(self.curve.shape.name()).to_string();
        self.curve.switch_shape(&next);
        self.clear_all_outputs();
        format!("ok shape={}\n", next)
    }
}

// ---------------------------------------------------------------------------
// Wayland protocol handlers
// ---------------------------------------------------------------------------

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.outputs.push((output.clone(), None));
        self.create_surface_for_output(&output);
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.outputs.retain(|(wl, _)| wl != &output);
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let mut found_idx = None;
        for (i, (_wl, osurface)) in self.outputs.iter_mut().enumerate() {
            if let Some(os) = osurface {
                if os.layer.wl_surface() == layer.wl_surface() {
                    let new_w = configure.new_size.0.max(1);
                    let new_h = configure.new_size.1.max(1);
                    os.width = new_w;
                    os.height = new_h;
                    os._wl_egl_surface.resize(new_w as i32, new_h as i32, 0, 0);
                    os.configured = true;
                    found_idx = Some(i);
                    info!("Layer surface configured: {}x{}", new_w, new_h);
                    break;
                }
            }
        }
        if let Some(idx) = found_idx {
            self.init_renderer_for_output(idx);
            self.update_scales();
        }
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_layer!(App);
delegate_registry!(App);

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

// ---------------------------------------------------------------------------
// EGL initialization
// ---------------------------------------------------------------------------

fn init_egl(
    conn: &Connection,
) -> (
    khronos_egl::Instance<khronos_egl::Static>,
    khronos_egl::Display,
    khronos_egl::Context,
    khronos_egl::Config,
) {
    let egl = khronos_egl::Instance::new(khronos_egl::Static);

    let wl_display = conn.backend().display_ptr() as *mut std::ffi::c_void;

    // Get EGL display from Wayland display
    let egl_display = unsafe {
        egl.get_display(wl_display as khronos_egl::NativeDisplayType)
            .expect("get EGL display")
    };
    egl.initialize(egl_display).expect("EGL initialize");

    let attributes = [
        khronos_egl::RED_SIZE,
        8,
        khronos_egl::GREEN_SIZE,
        8,
        khronos_egl::BLUE_SIZE,
        8,
        khronos_egl::ALPHA_SIZE,
        8,
        khronos_egl::SURFACE_TYPE,
        khronos_egl::WINDOW_BIT,
        khronos_egl::RENDERABLE_TYPE,
        khronos_egl::OPENGL_ES2_BIT,
        khronos_egl::NONE,
    ];

    let config = egl
        .choose_first_config(egl_display, &attributes)
        .expect("choose EGL config")
        .expect("no matching EGL config");

    let context_attrs = [
        khronos_egl::CONTEXT_MAJOR_VERSION,
        2,
        khronos_egl::CONTEXT_MINOR_VERSION,
        0,
        khronos_egl::NONE,
    ];

    let context = egl
        .create_context(egl_display, config, None, &context_attrs)
        .expect("create EGL context");

    (egl, egl_display, context, config)
}

// ---------------------------------------------------------------------------
// Mode detection
// ---------------------------------------------------------------------------

/// Should we run in ASCII terminal mode?
///
/// True when:
///   - `--ascii` or `-a` flag is passed
///   - `WALLS_MODE=ascii` env var is set
///   - `WAYLAND_DISPLAY` is unset (no compositor available)
fn should_use_ascii() -> bool {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--ascii" || a == "-a") {
        return true;
    }
    if let Ok(mode) = env::var("WALLS_MODE") {
        if mode.eq_ignore_ascii_case("ascii") {
            return true;
        }
    }
    env::var("WAYLAND_DISPLAY").map_or(true, |v| v.is_empty())
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    env_logger::init();

    if should_use_ascii() {
        wl_walls::ascii::run();
        return;
    }

    run_wayland();
}

fn run_wayland() {
    let conn = Connection::connect_to_env().expect("Failed to connect to Wayland");
    let (globals, mut event_queue) = registry_queue_init(&conn).expect("registry init");
    let qh = event_queue.handle();

    let compositor_state =
        CompositorState::bind(&globals, &qh).expect("wl_compositor not available");
    let layer_shell = LayerShell::bind(&globals, &qh).expect("wlr-layer-shell not available");
    let shm = Shm::bind(&globals, &qh).expect("wl_shm not available");
    let output_state = OutputState::new(&globals, &qh);
    let registry_state = RegistryState::new(&globals);

    let (egl, egl_display, egl_context, egl_config) = init_egl(&conn);

    let (fg_colors, bg_color) = colors_from_env();
    let mut rng = rand::thread_rng();
    let current_color = fg_colors[rng.gen_range(0..fg_colors.len())];

    let steps_per_tick = parse_env_u32("WALLS_SPEED", 1).max(1);
    let fps = parse_env_u32("WALLS_FPS", 30).clamp(1, 144);
    let fade_amount = parse_env_f32("WALLS_FADE", 0.005).max(0.0);
    let line_width = parse_env_f64("WALLS_LINE_WIDTH", 2.0).max(0.5);
    let line_alpha = parse_env_f32("WALLS_ALPHA", 0.85).clamp(0.01, 1.0);
    let dither_strength = parse_env_f32("WALLS_DITHER", 0.0).clamp(0.0, 1.0);
    let dither_levels = parse_env_f32("WALLS_DITHER_LEVELS", 8.0).clamp(2.0, 256.0);
    let dither_scale = parse_env_f32("WALLS_DITHER_SCALE", 1.0).max(1.0);
    let frame_interval = Duration::from_millis((1000 / fps as u64).max(1));

    let (shape_lock, initial_shape) = resolve_shape_env();

    info!(
        "Config: {}fps, speed={}, fade={}, line_width={}, alpha={}, dither={}/{}/{}, shape={}",
        fps, steps_per_tick, fade_amount, line_width, line_alpha,
        dither_strength, dither_levels, dither_scale,
        initial_shape.name(),
    );

    let control = match ControlSocket::bind() {
        Ok(c) => Some(c),
        Err(e) => {
            warn!("Could not open control socket: {}", e);
            None
        }
    };

    let mut app = App {
        registry_state,
        output_state,
        compositor_state,
        layer_shell,
        shm,
        qh: qh.clone(),
        egl,
        egl_display,
        egl_context,
        egl_config,
        control,
        outputs: Vec::new(),
        curve: CurveDrawer::new(initial_shape),
        shape_lock,
        fg_colors,
        bg_color,
        current_color,
        steps_per_tick,
        fade_amount,
        line_width,
        line_alpha,
        dither_strength,
        dither_levels,
        dither_scale,
        scale_x: 0.4,
        scale_y: 0.4,
    };

    event_queue.roundtrip(&mut app).expect("roundtrip");

    let mut event_loop: EventLoop<App> = EventLoop::try_new().expect("calloop event loop");
    let loop_handle = event_loop.handle();

    WaylandSource::new(conn.clone(), event_queue)
        .insert(loop_handle.clone())
        .expect("insert wayland source");

    loop_handle
        .insert_source(
            Timer::from_duration(frame_interval),
            move |_, _, app| {
                app.tick();
                TimeoutAction::ToDuration(frame_interval)
            },
        )
        .expect("insert timer");

    info!("Starting event loop (GPU-accelerated)");
    loop {
        event_loop.dispatch(None, &mut app).expect("dispatch");
    }
}
