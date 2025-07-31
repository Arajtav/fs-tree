mod colors;
mod extensions;
mod render_tree;

use std::{
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use clap::Parser;
use fs_tree_shared::scan_dir;
use render_tree::{ColorMode, RenderTree};
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

fn highest_aspect_ratio(row: &[u64], row_area: f32, w: f32, h: f32) -> f32 {
    if row.is_empty() || row_area == 0.0 {
        return f32::INFINITY;
    }

    let total_size = row.iter().sum::<u64>() as f32;
    row.iter()
        .map(|&child| {
            let child = child as f32;

            let (w, h) = if w >= h {
                (row_area / h, child * h / total_size)
            } else {
                (child * w / total_size, row_area / w)
            };

            (w / h).max(h / w)
        })
        .fold(0.0, |acc, x| acc.max(x))
}

fn recursive_compute_layout(
    tree: &RenderTree,
    mut x: f32,
    mut y: f32,
    mut dx: f32,
    mut dy: f32,
    aspect_ratio: f32,
    out: &mut Vec<Instance>,
) {
    match tree {
        RenderTree::File { color, .. } => {
            out.push(Instance {
                position: [x, y * aspect_ratio],
                size: [dx, dy * aspect_ratio],
                color: [color[0], color[1], color[2], 1.0],
            });
        }

        RenderTree::Dir { size, children, .. } => {
            let size = *size as f32;

            let total_area = dx * dy;

            let mut base_index = 0;

            while base_index < children.len() {
                let mut current_total = children[base_index].get_size();
                while current_total == 0 && base_index < children.len() {
                    base_index += 1;
                    current_total = children[base_index].get_size();
                }

                if base_index >= children.len() {
                    break;
                }

                let mut current_row: Vec<&RenderTree> = vec![&children[base_index]];
                let mut offset_index = 1;

                while base_index + offset_index < children.len() {
                    let current_child = &children[base_index + offset_index];
                    let current_child_size = current_child.get_size();

                    debug_assert_ne!(current_child_size, 0);

                    let mut sizes: Vec<u64> = current_row.iter().map(|e| e.get_size()).collect();

                    let current_aspect = highest_aspect_ratio(
                        &sizes,
                        total_area * (current_total as f32 / size),
                        dx,
                        dy,
                    );

                    sizes.push(current_child_size);
                    let next_total = current_total + current_child_size;

                    let next_aspect = highest_aspect_ratio(
                        &sizes,
                        total_area * (next_total as f32 / size),
                        dx,
                        dy,
                    );

                    if next_aspect > current_aspect {
                        break;
                    }

                    current_row.push(current_child);
                    current_total = next_total;
                    offset_index += 1;
                }

                let current_total = current_total as f32;

                let row_length = total_area * current_total / (size * dx.min(dy));

                let mut offset = 0.0;
                for child in current_row.iter() {
                    let child_size = child.get_size() as f32;
                    if dx >= dy {
                        recursive_compute_layout(
                            child,
                            x,
                            y + offset,
                            row_length,
                            dy * child_size / current_total,
                            aspect_ratio,
                            out,
                        );
                        offset += dy * child_size / current_total;
                    } else {
                        recursive_compute_layout(
                            child,
                            x + offset,
                            y,
                            dx * child_size / current_total,
                            row_length,
                            aspect_ratio,
                            out,
                        );
                        offset += dx * child_size / current_total;
                    }
                }

                if dx >= dy {
                    x += row_length;
                    dx -= row_length;
                } else {
                    y += row_length;
                    dy -= row_length;
                }
                base_index += offset_index;
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

        // I guess the size could be { 0, 0 }, shouldn't break anything though.
        let mut instances = Vec::new();
        recursive_compute_layout(
            &self.data,
            0.0,
            0.0,
            1.0,
            size.height as f32 / size.width as f32,
            size.width as f32 / size.height as f32,
            &mut instances,
        );
        dbg!(instances.len());

        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instances),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
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
            // there is enough delay for at least one frame to render before the layout changes
            // apparently it is a wayland problem, but it would be really nice if there was a way to fix it
            WindowEvent::Resized(new_size) => {
                let render_data = match self.render_data.as_mut() {
                    Some(render_data) => render_data,
                    None => {
                        return;
                    }
                };
                let width = new_size.width.max(1);
                let height = new_size.height.max(1);
                render_data.config.width = width;
                render_data.config.height = height;

                let mut instances = Vec::new();
                recursive_compute_layout(
                    &self.data,
                    0.0,
                    0.0,
                    1.0,
                    height as f32 / width as f32,
                    width as f32 / height as f32,
                    &mut instances,
                );

                assert_eq!(
                    (size_of::<Instance>() * instances.len()) as u64,
                    render_data.instance_buffer.size(),
                );

                render_data.queue.write_buffer(
                    &render_data.instance_buffer,
                    0,
                    bytemuck::cast_slice(&instances),
                );

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
    #[clap(default_value = "extension")]
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
        nix::unistd::getuid().into(),
        nix::unistd::getgid().into(),
    );

    let mut app = App::new(render_tree);
    let _ = event_loop.run_app(&mut app);
}
