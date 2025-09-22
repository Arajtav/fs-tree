mod colors;
mod extensions;
mod render_tree;
mod utils;

use std::{
    ffi::OsString,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use ab_glyph::FontArc;
use clap::Parser;
use fs_tree_shared::scan_dir;
use glyph_brush::{HorizontalAlign, Layout, Section, Text, VerticalAlign};
use render_tree::{ColorMode, RenderTree};
use wgpu::{util::DeviceExt, DeviceDescriptor, PowerPreference, SurfaceConfiguration};
use wgpu_text::{BrushBuilder, TextBrush};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::utils::{entry_description, get_font};

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

fn highest_aspect_ratio(row: &[u64], row_area: f32, size: (f32, f32)) -> f32 {
    if row.is_empty() || row_area == 0.0 {
        return f32::INFINITY;
    }

    let total_size = row.iter().sum::<u64>() as f32;
    row.iter()
        .map(|&child| {
            let child = child as f32;

            let (w, h) = if size.0 >= size.1 {
                (row_area / size.1, child * size.1 / total_size)
            } else {
                (child * size.0 / total_size, row_area / size.0)
            };

            (w / h).max(h / w)
        })
        .fold(0.0, |acc, x| acc.max(x))
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Rectangle {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
}

impl Rectangle {
    fn overlaps(&self, x: f32, y: f32) -> bool {
        x >= self.x && y >= self.y && x <= self.x + self.dx && y <= self.y + self.dy
    }

    fn get_center(&self) -> (f32, f32) {
        (self.x + self.dx * 0.5, self.y + self.dy * 0.5)
    }

    fn to_vertices(self) -> [Vertex; 4] {
        [
            Vertex {
                position: [self.x, self.y],
            },
            Vertex {
                position: [self.x + self.dx, self.y],
            },
            Vertex {
                position: [self.x, self.y + self.dy],
            },
            Vertex {
                position: [self.x + self.dx, self.y + self.dy],
            },
        ]
    }
}

fn recursive_compute_layout(
    tree: &RenderTree,
    mut base_position: (f32, f32),
    mut base_size: (f32, f32),
    aspect_ratio: f32,
    out: &mut Vec<Instance>,
    mut level_out: Option<&mut Vec<(Rectangle, OsString, u64, bool)>>,
) {
    match tree {
        RenderTree::File { color, .. } => {
            out.push(Instance {
                position: [base_position.0, base_position.1 * aspect_ratio],
                size: [base_size.0, base_size.1 * aspect_ratio],
                color: [color[0], color[1], color[2], 1.0],
            });
        }

        RenderTree::Dir { size, children, .. } => {
            let size = *size as f32;

            let total_area = base_size.0 * base_size.1;

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
                        base_size,
                    );

                    sizes.push(current_child_size);
                    let next_total = current_total + current_child_size;

                    let next_aspect = highest_aspect_ratio(
                        &sizes,
                        total_area * (next_total as f32 / size),
                        base_size,
                    );

                    if next_aspect > current_aspect {
                        break;
                    }

                    current_row.push(current_child);
                    current_total = next_total;
                    offset_index += 1;
                }

                let current_total = current_total as f32;

                let row_length = total_area * current_total / (size * base_size.0.min(base_size.1));

                let mut offset = 0.0;
                for child in current_row.iter() {
                    let child_size = child.get_size() as f32;
                    let (child_x, child_y, child_dx, child_dy) = if base_size.0 >= base_size.1 {
                        let height = base_size.1 * child_size / current_total;
                        let pos = (
                            base_position.0,
                            base_position.1 + offset,
                            row_length,
                            height,
                        );
                        offset += height;
                        pos
                    } else {
                        let width = base_size.0 * child_size / current_total;
                        let pos = (base_position.0 + offset, base_position.1, width, row_length);
                        offset += width;
                        pos
                    };

                    if let Some(ref mut level_out) = level_out {
                        level_out.push((
                            Rectangle {
                                x: child_x,
                                y: child_y * aspect_ratio,
                                dx: child_dx,
                                dy: child_dy * aspect_ratio,
                            },
                            child.get_name().to_owned(),
                            child.get_size(),
                            matches!(child, RenderTree::File { .. }),
                        ))
                    }

                    recursive_compute_layout(
                        child,
                        (child_x, child_y),
                        (child_dx, child_dy),
                        aspect_ratio,
                        out,
                        None,
                    );
                }

                if base_size.0 >= base_size.1 {
                    base_position.0 += row_length;
                    base_size.0 -= row_length;
                } else {
                    base_position.1 += row_length;
                    base_size.1 -= row_length;
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
    cursor: Option<(f32, f32)>,
    pipeline2: wgpu::RenderPipeline,
    vertex_buffer2: wgpu::Buffer,
    brush: TextBrush,
}

struct App {
    render_data: Option<RenderData>,
    data: Vec<&'static RenderTree>,
    path: PathBuf,
    base: PathBuf,
    current: usize,
    level: Vec<(Rectangle, OsString, u64, bool)>,
    font: FontArc,
}

impl App {
    fn new(render_tree: &'static RenderTree, font: FontArc, base: PathBuf) -> Self {
        Self {
            render_data: None,
            data: vec![render_tree],
            path: PathBuf::new(),
            base,
            current: 0,
            level: Vec::new(),
            font,
        }
    }

    fn regenerate_layout_and_request_redraw(&mut self) {
        let render_data = self.render_data.as_mut().unwrap();
        let size = render_data.window.inner_size();
        let width = size.width.max(1) as f32;
        let height = size.height.max(1) as f32;

        let mut instances = Vec::new();
        let mut level = Vec::new();

        recursive_compute_layout(
            self.data[self.current],
            (0.0, 0.0),
            (1.0, height / width),
            width / height,
            &mut instances,
            Some(&mut level),
        );

        self.level = level;

        render_data.instance_count = instances.len() as u32;

        render_data.queue.write_buffer(
            &render_data.instance_buffer,
            0,
            bytemuck::cast_slice(&instances),
        );

        render_data.window.request_redraw();
    }

    fn go_back(&mut self) {
        if self.current > 0 {
            self.current -= 1;
            self.regenerate_layout_and_request_redraw();
        }
    }

    fn go_forward(&mut self) {
        if self.current + 1 < self.data.len() {
            self.current += 1;
            self.regenerate_layout_and_request_redraw();
        }
    }

    fn go_next(&mut self, next: &'static RenderTree, name: OsString) {
        if self.data.get(self.current + 1).map(|r| *r as *const _) != Some(next as *const _) {
            self.data.truncate(self.current + 1);
            self.data.push(next);

            let mut new_path = self
                .path
                .components()
                .take(self.current)
                .collect::<PathBuf>();

            new_path.push(name);
            self.path = new_path;
        }
        self.current += 1;

        self.regenerate_layout_and_request_redraw();
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
        let mut level = Vec::new();
        recursive_compute_layout(
            self.data[self.current],
            (0.0, 0.0),
            (1.0, size.height as f32 / size.width as f32),
            size.width as f32 / size.height as f32,
            &mut instances,
            Some(&mut level),
        );
        dbg!(instances.len());
        dbg!(level.len());

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
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let pipeline_layout2 = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout 2"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let vertex_buffer2 = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer 2"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let shader2 = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader 2"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader2.wgsl").into()),
        });

        let pipeline2 = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipeline 2"),
            cache: Default::default(),
            layout: Some(&pipeline_layout2),
            vertex: wgpu::VertexState {
                compilation_options: Default::default(),
                module: &shader2,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                compilation_options: Default::default(),
                module: &shader2,
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

        self.level = level;

        let brush = BrushBuilder::using_font(self.font.clone()).build(
            &device,
            config.width,
            config.height,
            config.format,
        );

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
            cursor: None,
            pipeline2,
            vertex_buffer2,
            brush,
        });

        self.render_data.as_ref().unwrap().window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let render_data = match self.render_data.as_mut() {
            Some(render_data) => render_data,
            None => {
                return;
            }
        };
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            // there is enough delay for at least one frame to render before the layout changes
            // apparently it is a wayland problem, but it would be really nice if there was a way to fix it
            WindowEvent::Resized(new_size) => {
                let width = new_size.width.max(1);
                let height = new_size.height.max(1);
                render_data.config.width = width;
                render_data.config.height = height;

                render_data
                    .brush
                    .resize_view(width as f32, height as f32, &render_data.queue);

                render_data
                    .surface
                    .configure(&render_data.device, &render_data.config);

                self.regenerate_layout_and_request_redraw();
            }
            WindowEvent::RedrawRequested => {
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

                if let Some((x, y)) = render_data.cursor {
                    if let Some((rect, name, size, is_file)) =
                        self.level.iter().find(|e| e.0.overlaps(x, y))
                    {
                        render_pass.set_pipeline(&render_data.pipeline2);
                        render_data.queue.write_buffer(
                            &render_data.vertex_buffer2,
                            0,
                            bytemuck::cast_slice(&rect.to_vertices()),
                        );
                        render_pass.set_vertex_buffer(0, render_data.vertex_buffer2.slice(..));
                        render_pass.set_index_buffer(
                            render_data.index_buffer.slice(..),
                            wgpu::IndexFormat::Uint16,
                        );
                        render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);

                        let text_str = entry_description(name, *is_file, *size);
                        let scale_x = render_data.config.width as f32 / 1920.0;
                        let scale_y = render_data.config.height as f32 / 1080.0;
                        let text_scale = 32.0 * scale_x.min(scale_y);
                        let text_scale = text_scale.clamp(12.0, 96.0);
                        let text = Text::new(&text_str).with_scale(text_scale);
                        let pos = rect.get_center();
                        let mut pos = (
                            pos.0 * render_data.config.width as f32,
                            (1.0 - pos.1) * render_data.config.height as f32,
                        );
                        let section = Section::default()
                            .add_text(text)
                            .with_screen_position(pos)
                            .with_layout(
                                Layout::default()
                                    .h_align(HorizontalAlign::Center)
                                    .v_align(VerticalAlign::Center),
                            );

                        const PADDING: f32 = 12.0;
                        let bounds = render_data.brush.glyph_bounds(&section).unwrap();
                        if bounds.min.x < PADDING {
                            pos.0 -= bounds.min.x - PADDING;
                        } else if bounds.max.x > render_data.config.width as f32 - PADDING {
                            pos.0 -= bounds.max.x - (render_data.config.width as f32 - PADDING);
                        }
                        if bounds.min.y < PADDING {
                            pos.1 -= bounds.min.y - PADDING;
                        } else if bounds.max.y > render_data.config.height as f32 - PADDING {
                            pos.1 -= bounds.max.y - (render_data.config.height as f32 - PADDING);
                        }

                        render_data
                            .brush
                            .queue(
                                &render_data.device,
                                &render_data.queue,
                                [&section.with_screen_position(pos)],
                            )
                            .unwrap();

                        render_data.brush.draw(&mut render_pass);
                    }
                }
                drop(render_pass);

                render_data.queue.submit(Some(encoder.finish()));
                output.present();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let size = render_data.window.inner_size();
                let x = position.x as f32 / size.width as f32;
                let y = position.y as f32 / size.height as f32;
                render_data.cursor = Some((x, 1.0 - y));
                render_data.window.request_redraw();
            }
            WindowEvent::CursorLeft { .. } => {
                render_data.cursor = None;
                render_data.window.request_redraw();
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some((x, y)) = render_data.cursor {
                    if let Some((_, name, ..)) = self.level.iter().find(|e| e.0.overlaps(x, y)) {
                        let children = match self.data[self.current] {
                            RenderTree::Dir { children, .. } => children,
                            RenderTree::File { .. } => unreachable!(),
                        };
                        let child = children.iter().find(|e| e.get_name() == name).unwrap();
                        if let RenderTree::Dir { .. } = child {
                            self.go_next(child, name.clone());
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } => {
                if let Some((x, y)) = render_data.cursor {
                    if let Some((_, name, ..)) = self.level.iter().find(|e| e.0.overlaps(x, y)) {
                        let path = self
                            .base
                            .join(
                                self.path
                                    .components()
                                    .take(self.current)
                                    .collect::<PathBuf>(),
                            )
                            .join(name);
                        if open::that_detached(&path).is_err() {
                            eprintln!("failed to open {path:?}")
                        }
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Back,
                ..
            } => self.go_back(),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Forward,
                ..
            } => self.go_forward(),

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

    let font = get_font().expect("Could not find any font.");

    let render_tree = RenderTree::from_scan_tree(
        scan_dir(&args.entrypoint),
        &args.color,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        #[cfg(target_family = "unix")]
        nix::unistd::getuid().into(),
        #[cfg(target_family = "unix")]
        nix::unistd::getgid().into(),
    );

    let static_tree: &'static RenderTree = Box::leak(Box::new(render_tree));
    let mut app = App::new(static_tree, font, args.entrypoint);
    let _ = event_loop.run_app(&mut app);
}
