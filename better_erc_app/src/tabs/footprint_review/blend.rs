//! The footprint mixed into the datasheet page on the GPU: a wgpu pass (an egui paint callback) draws the
//! footprint's triangles with a shader that reads the page's own pixel under each one and mixes the footprint's
//! colour in by a blend mode: normal, multiply, screen or difference (where a footprint that covers the drawing
//! exactly goes dark). egui's own painter only mixes by alpha, so the other modes need the page's pixel, which only
//! a shader with the page as a texture has.
//!
//! [`register`] puts the pipeline into the renderer's callback resources once, for the renderer's target format;
//! [`available`] tells the UI whether that happened (no wgpu renderer: the blend modes other than normal are off,
//! and say why).

use eframe::egui_wgpu::{self, wgpu};
use egui::{Color32, Mesh, Rect};
use std::sync::Arc;
use wgpu::util::DeviceExt as _;

pub use crate::settings::BlendMode;

fn code(m: BlendMode) -> u32 {
    match m {
        BlendMode::Normal => 0,
        BlendMode::Multiply => 1,
        BlendMode::Screen => 2,
        BlendMode::Difference => 3,
    }
}

const SHADER: &str = r#"
struct Params {
    // The callback's rectangle in points: min x, min y, width, height
    viewport: vec4<f32>,
    // The page's rectangle in points
    page: vec4<f32>,
    // The colour around the page, gamma
    around: vec4<f32>,
    opacity: f32,
    mode: u32,
    // 1: the target is an sRGB format and wants linear colour
    linear_out: u32,
    _pad: u32,
};

@group(0) @binding(0) var<uniform> p: Params;
@group(0) @binding(1) var page_tex: texture_2d<f32>;
@group(0) @binding(2) var page_smp: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) pos: vec2<f32>,
    @location(1) color: vec4<f32>,
};

fn unpack(c: u32) -> vec4<f32> {
    return vec4<f32>(f32(c & 255u), f32((c >> 8u) & 255u), f32((c >> 16u) & 255u), f32((c >> 24u) & 255u)) / 255.0;
}

@vertex
fn vs_main(@location(0) pos: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: u32) -> VsOut {
    var o: VsOut;
    let n = (pos - p.viewport.xy) / p.viewport.zw;
    o.clip = vec4<f32>(n.x * 2.0 - 1.0, 1.0 - n.y * 2.0, 0.0, 1.0);
    o.pos = pos;
    o.color = unpack(color);
    return o;
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let lo = c / 12.92;
    let hi = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(hi, lo, c <= vec3<f32>(0.04045));
}

@fragment
fn fs_main(v: VsOut) -> @location(0) vec4<f32> {
    let uv = (v.pos - p.page.xy) / p.page.zw;
    var base = p.around.rgb;
    if (all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0))) {
        base = textureSampleLevel(page_tex, page_smp, uv, 0.0).rgb;
    }
    // Vertex colours are premultiplied by the edge's coverage
    let a = v.color.a;
    var c = vec3<f32>(0.0);
    if (a > 0.0) {
        c = v.color.rgb / a;
    }
    var mixed = c;
    if (p.mode == 1u) {
        mixed = base * c;
    } else if (p.mode == 2u) {
        mixed = base + c - base * c;
    } else if (p.mode == 3u) {
        mixed = abs(base - c);
    }
    var out = mix(base, mixed, a * p.opacity);
    if (p.linear_out == 1u) {
        out = to_linear(out);
    }
    return vec4<f32>(out, 1.0);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    viewport: [f32; 4],
    page: [f32; 4],
    around: [f32; 4],
    opacity: f32,
    mode: u32,
    linear_out: u32,
    _pad: u32,
}

/// The pipeline, kept in the renderer's callback resources, and what each review tab draws with
struct Resources {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    linear_out: bool,
    views: std::collections::HashMap<egui::Id, View>,
}

/// One review tab's page and triangles
struct View {
    params: wgpu::Buffer,
    /// The page picture uploaded last, by its generation
    page: Option<(u64, wgpu::BindGroup)>,
    /// This frame's triangles
    vertices: Option<(wgpu::Buffer, wgpu::Buffer, u32)>,
}

/// The id under which [`register`] notes in egui's memory that the blend pass is there
fn flag_id() -> egui::Id {
    egui::Id::new("footprint review blend pass")
}

/// Whether the blend pass was registered for this egui context's renderer
pub fn available(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(flag_id()).unwrap_or(false))
}

/// Sets the blend pass up for a wgpu renderer drawing into `format` with `msaa` samples and `depth` (the renderer's
/// own settings), and notes it in `ctx`
pub fn register(
    ctx: &egui::Context,
    rs: &egui_wgpu::RenderState,
    msaa: u32,
    depth: Option<wgpu::TextureFormat>,
) {
    let device = &rs.device;
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("footprint blend"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("footprint blend"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("footprint blend"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("footprint blend"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 5 * 4,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Uint32],
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: depth.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: msaa.max(1),
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            // The shader mixes with the page itself: it writes the result, no blending by the GPU
            targets: &[Some(wgpu::ColorTargetState {
                format: rs.target_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("footprint blend page"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    });
    rs.renderer.write().callback_resources.insert(Resources {
        pipeline,
        layout,
        sampler,
        linear_out: rs.target_format.is_srgb(),
        views: Default::default(),
    });
    ctx.data_mut(|d| d.insert_temp(flag_id(), true));
}

/// The page picture the pass reads, with a generation that changes with the picture
#[derive(Clone)]
pub struct PageImage {
    pub generation: u64,
    pub image: Arc<egui::ColorImage>,
}

/// One frame's blend pass: the footprint's triangles over the page
pub struct BlendPass {
    /// The review tab drawing
    pub id: egui::Id,
    pub mesh: Mesh,
    pub page: PageImage,
    /// Where the page is, in points
    pub page_rect: Rect,
    /// Around the page
    pub around: Color32,
    pub opacity: f32,
    pub mode: BlendMode,
    /// The callback's rectangle (the canvas), in points
    pub viewport: Rect,
}

impl egui_wgpu::CallbackTrait for BlendPass {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(res) = resources.get_mut::<Resources>() else {
            return Vec::new();
        };
        let linear_out = res.linear_out;
        let r = res.views.entry(self.id).or_insert_with(|| View {
            params: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("footprint blend params"),
                size: std::mem::size_of::<Params>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            page: None,
            vertices: None,
        });
        if r.page
            .as_ref()
            .is_none_or(|(g, _)| *g != self.page.generation)
        {
            let [w, h] = self.page.image.size;
            let size = wgpu::Extent3d {
                width: w as u32,
                height: h as u32,
                depth_or_array_layers: 1,
            };
            // Gamma values as they are: the shader mixes in gamma space, as egui and image editors do
            let texture = device.create_texture_with_data(
                queue,
                &wgpu::TextureDescriptor {
                    label: Some("footprint blend page"),
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                },
                wgpu::util::TextureDataOrder::LayerMajor,
                bytemuck::cast_slice(&self.page.image.pixels),
            );
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("footprint blend"),
                layout: &res.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: r.params.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&res.sampler),
                    },
                ],
            });
            r.page = Some((self.page.generation, bind));
        }
        let rect = |r: Rect| [r.min.x, r.min.y, r.width(), r.height()];
        let around = self.around.to_normalized_gamma_f32();
        let params = Params {
            viewport: rect(self.viewport),
            page: rect(self.page_rect),
            around,
            opacity: self.opacity,
            mode: code(self.mode),
            linear_out: linear_out as u32,
            _pad: 0,
        };
        queue.write_buffer(&r.params, 0, bytemuck::bytes_of(&params));
        r.vertices = (!self.mesh.indices.is_empty()).then(|| {
            let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("footprint blend vertices"),
                contents: bytemuck::cast_slice(&self.mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("footprint blend indices"),
                contents: bytemuck::cast_slice(&self.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });
            (vb, ib, self.mesh.indices.len() as u32)
        });
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(res) = resources.get::<Resources>() else {
            return;
        };
        let Some(r) = res.views.get(&self.id) else {
            return;
        };
        let (Some((_, bind)), Some((vb, ib, n))) = (&r.page, &r.vertices) else {
            return;
        };
        pass.set_pipeline(&res.pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..*n, 0, 0..1);
    }
}
