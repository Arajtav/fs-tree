use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use fs_tree::{
    render_tree::{ColorMode, RenderTree},
    scan_tree::scan_dir,
};
use wgpu::{util::DeviceExt, DeviceDescriptor, PowerPreference, SurfaceConfiguration};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.0],
    },
    Vertex {
        position: [1.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
    },
];

const INDICES: &[u16] = &[0, 1, 2, 2, 1, 3];

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    position: [f32; 2],
    size: [f32; 2],
    color: [f32; 4],
}

fn recursive_compute_layout(
    tree: &RenderTree,
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    horizontal: bool,
    out: &mut Vec<Instance>,
) {
    match tree {
        RenderTree::File { color, .. } => {
            out.push(Instance {
                position: [x, y],
                size: [dx, dy],
                color: *color,
            });
        }

        RenderTree::Dir { size, children, .. } => {
            let mut offset = 0.0;
            for child in children {
                let child_size = child.get_size();
                if child_size == 0 {
                    continue;
                }

                if horizontal {
                    let child_dx = dx * child_size as f32 / *size as f32;
                    recursive_compute_layout(child, x + offset, y, child_dx, dy, false, out);
                    offset += child_dx;
                } else {
                    let child_dy = dy * child_size as f32 / *size as f32;
                    recursive_compute_layout(child, x, y + offset, dx, child_dy, true, out);
                    offset += child_dy;
                }
            }
        }
    }
}

struct RenderData {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
}

struct App {
    render_data: Option<RenderData>,
    data: RenderTree,
}

impl App {
    fn new(render_tree: RenderTree) -> Self {
        Self {
            render_data: None,
            data: render_tree,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(winit::window::Window::default_attributes().with_title("fs-tree"))
                .unwrap(),
        );

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        }))
        .unwrap();

        let device_desc = DeviceDescriptor {
            label: Some("Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::defaults(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        };

        let (device, queue) = pollster::block_on(adapter.request_device(&device_desc)).unwrap();

        let surface_format = *surface.get_capabilities(&adapter).formats.first().unwrap();
        let size = window.inner_size();

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        let mut instances = Vec::new();
        recursive_compute_layout(&self.data, 0.0, 0.0, 1.0, 1.0, true, &mut instances);

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            cache: Default::default(),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[
                    // Vertex
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    // Instance
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Instance>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x2, // position
                            2 => Float32x2, // size
                            3 => Float32x4  // color
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        self.render_data = Some(RenderData {
            vertex_buffer,
            index_buffer,
            instance_buffer,
            instance_count: instances.len() as u32,
            pipeline,
            surface,
            device,
            queue,
            config,
            window,
        });

        self.render_data.as_ref().unwrap().window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(new_size) => {
                let render_data = match self.render_data.as_mut() {
                    Some(render_data) => render_data,
                    None => {
                        return;
                    }
                };
                render_data.config.width = new_size.width.max(1);
                render_data.config.height = new_size.height.max(1);

                render_data
                    .surface
                    .configure(&render_data.device, &render_data.config);
            }
            WindowEvent::RedrawRequested => {
                let render_data = match self.render_data.as_ref() {
                    Some(render_data) => render_data,
                    None => {
                        return;
                    }
                };

                let output = match render_data.surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => return,
                };

                let view = output
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder =
                    render_data
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Encoder"),
                        });

                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        depth_slice: None,
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });

                render_pass.set_pipeline(&render_data.pipeline);
                render_pass.set_vertex_buffer(0, render_data.vertex_buffer.slice(..));
                render_pass.set_vertex_buffer(1, render_data.instance_buffer.slice(..));
                render_pass.set_index_buffer(
                    render_data.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint16,
                );
                render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..render_data.instance_count);
                drop(render_pass);

                render_data.queue.submit(Some(encoder.finish()));
                output.present();
            }
            _ => {}
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    /// Location from where to start the scan.
    entrypoint: PathBuf,

    /// Color mode
    #[cfg(feature = "full_metadata")]
    #[clap(default_value = "access")]
    color: ColorMode,

    /// Color mode
    #[cfg(not(feature = "full_metadata"))]
    #[clap(default_value = "debug")]
    color: ColorMode,
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let args = Args::parse();
    let render_tree = RenderTree::from_scan_tree(
        scan_dir(&args.entrypoint),
        &args.color,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
    );

    let mut app = App::new(render_tree);
    let _ = event_loop.run_app(&mut app);
}
