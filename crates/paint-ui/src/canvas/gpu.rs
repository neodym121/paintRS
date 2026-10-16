//! wgpu renderer for the canvas: one texture with a full mip chain (uploaded from the CPU),
//! drawn through an egui paint callback.

use std::num::NonZeroU64;

use egui_wgpu::wgpu;
use egui_wgpu::{CallbackResources, CallbackTrait, RenderState};
use paint_core::IRect;

use super::mips::MipChain;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Uniforms {
    pub checker_a: [f32; 4],
    pub checker_b: [f32; 4],
    pub grid_color: [f32; 4],
    pub canvas_origin: [f32; 2],
    pub canvas_size: [f32; 2],
    pub tex_size: [f32; 2],
    pub checker_px: f32,
    pub grid_alpha: f32,
    pub srgb_target: f32,
    pub _pad: [f32; 3],
}

/// GPU objects shared with the paint callback (stored in egui-wgpu's callback resources).
struct Resources {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    uniforms: wgpu::Buffer,
    linear: wgpu::Sampler,
    nearest: wgpu::Sampler,
    bind_linear: Option<wgpu::BindGroup>,
    bind_nearest: Option<wgpu::BindGroup>,
}

/// UI-side handle: owns the texture and uploads changed regions.
pub struct CanvasGpu {
    state: RenderState,
    texture: Option<wgpu::Texture>,
    /// Size of the uploaded base level and which CPU mip level it comes from
    /// (non-zero when the image is larger than the GPU allows).
    tex_size: (u32, u32),
    base_level: usize,
    max_dim: u32,
    srgb_target: bool,
}

impl CanvasGpu {
    pub fn new(state: &RenderState) -> Self {
        let device = &state.device;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("paintrs canvas"),
            source: wgpu::ShaderSource::Wgsl(include_str!("canvas.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("paintrs canvas"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: NonZeroU64::new(std::mem::size_of::<Uniforms>() as u64),
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
            label: Some("paintrs canvas"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("paintrs canvas"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: state.target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("paintrs canvas uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = |filter: wgpu::FilterMode, mip: wgpu::MipmapFilterMode| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("paintrs canvas"),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: mip,
                ..Default::default()
            })
        };
        let linear = sampler(wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Linear);
        let nearest = sampler(wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Nearest);
        state.renderer.write().callback_resources.insert(Resources {
            pipeline,
            layout,
            uniforms,
            linear,
            nearest,
            bind_linear: None,
            bind_nearest: None,
        });
        Self {
            state: state.clone(),
            texture: None,
            tex_size: (0, 0),
            base_level: 0,
            max_dim: state.device.limits().max_texture_dimension_2d,
            srgb_target: state.target_format.is_srgb(),
        }
    }

    pub fn srgb_target(&self) -> bool {
        self.srgb_target
    }

    /// (Re)creates the texture for `mips` and uploads every level.
    pub fn recreate(&mut self, mips: &MipChain) {
        let mut base = 0;
        while base + 1 < mips.len() {
            let l = mips.level(base);
            if l.width() <= self.max_dim && l.height() <= self.max_dim {
                break;
            }
            base += 1;
        }
        let l0 = mips.level(base);
        let (w, h) = (l0.width(), l0.height());
        let levels = (mips.len() - base) as u32;
        let device = &self.state.device;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("paintrs canvas"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut renderer = self.state.renderer.write();
            if let Some(res) = renderer.callback_resources.get_mut::<Resources>() {
                let make = |sampler: &wgpu::Sampler| {
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("paintrs canvas"),
                        layout: &res.layout,
                        entries: &[
                            wgpu::BindGroupEntry { binding: 0, resource: res.uniforms.as_entire_binding() },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(&view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Sampler(sampler),
                            },
                        ],
                    })
                };
                res.bind_linear = Some(make(&res.linear));
                res.bind_nearest = Some(make(&res.nearest));
            }
        }
        self.texture = Some(texture);
        self.tex_size = (w, h);
        self.base_level = base;
        for i in base..mips.len() {
            self.upload_level(mips, i, mips.level(i).bounds());
        }
    }

    /// Uploads the regions returned by [`MipChain::update`].
    pub fn upload(&mut self, mips: &MipChain, changed: &[(usize, IRect)]) {
        for &(level, rect) in changed {
            if level >= self.base_level {
                self.upload_level(mips, level, rect);
            }
        }
    }

    fn upload_level(&self, mips: &MipChain, level: usize, rect: IRect) {
        let Some(texture) = &self.texture else { return };
        let src = mips.level(level);
        let r = rect.intersect(&src.bounds());
        if r.is_empty() {
            return;
        }
        let stride = src.width() * 4;
        self.state.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: (level - self.base_level) as u32,
                origin: wgpu::Origin3d { x: r.x as u32, y: r.y as u32, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            src.as_bytes(),
            wgpu::TexelCopyBufferLayout {
                offset: (r.y as u64 * src.width() as u64 + r.x as u64) * 4,
                bytes_per_row: Some(stride),
                rows_per_image: Some(r.h),
            },
            wgpu::Extent3d { width: r.w, height: r.h, depth_or_array_layers: 1 },
        );
    }

    pub fn has_texture(&self) -> bool {
        self.texture.is_some()
    }
}

/// Per-frame draw request.
pub struct CanvasCallback {
    pub uniforms: Uniforms,
    pub nearest: bool,
}

impl CallbackTrait for CanvasCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(res) = resources.get::<Resources>() {
            queue.write_buffer(&res.uniforms, 0, bytemuck::bytes_of(&self.uniforms));
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let Some(res) = resources.get::<Resources>() else { return };
        let bind = if self.nearest { &res.bind_nearest } else { &res.bind_linear };
        let Some(bind) = bind else { return };
        pass.set_pipeline(&res.pipeline);
        pass.set_bind_group(0, bind, &[]);
        pass.draw(0..3, 0..1);
    }
}
