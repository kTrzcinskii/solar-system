use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{
    camera, instance,
    sphere::{self, DrawSphere, Vertex},
    sun,
    texture::{self, SetTextureContainer},
};

struct State {
    last_render_time: Instant,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipeline: wgpu::RenderPipeline,
    sun_render_pipeline: wgpu::RenderPipeline,
    planets_texture_container: texture::TextureContainer,
    camera_container: camera::CameraContainer,
    instances: Vec<instance::Instance>,
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    sphere: sphere::Sphere,
    sun: sun::Sun,
    window: Arc<Window>,
}

impl State {
    async fn new(window: Arc<Window>) -> Result<Self> {
        let size = window.inner_size();

        // Handle to GPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let planets_texture_container =
            texture::TextureContainer::initialize_plantes_texture_array_container(&device, &queue);

        let camera_container = camera::CameraContainer::new(config.width, config.height, &device);

        const SPACE_BETWEEN: f32 = 3.0;
        let mut i = 0;
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = glam::Vec3 { x, y: 0.0, z } - INSTANCE_DISPLACEMENT;

                    let rotation = if position == glam::Vec3::ZERO {
                        // this is needed so an object at (0, 0, 0) won't get scaled to zero
                        // as Quaternions can affect scale if they're not created correctly
                        glam::Quat::from_axis_angle(glam::Vec3::Z, 0.0)
                    } else {
                        glam::Quat::from_axis_angle(position.normalize(), (45.0_f32).to_radians())
                    };

                    i += 1;
                    instance::Instance::new(position, rotation, i % 9)
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances
            .iter()
            .map(instance::InstanceRaw::from)
            .collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let sun = sun::Sun::new(&device, &queue, [2.0, 6.0, 2.0]);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &planets_texture_container.bind_group_layout,
                    &camera_container.camera_bind_group_layout,
                    &sun.light.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/main.wgsl").into()),
            };
            Self::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[sphere::SphereVertex::desc(), instance::InstanceRaw::desc()],
                shader,
            )
        };

        let sun_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sun Pipeline Layout"),
                bind_group_layouts: &[
                    &sun.texture_container.bind_group_layout,
                    &camera_container.camera_bind_group_layout,
                    &sun.light.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Sun Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/sun.wgsl").into()),
            };
            Self::create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[sphere::SphereVertex::desc()],
                shader,
            )
        };

        let sphere = sphere::Sphere::new(&device);

        Ok(State {
            last_render_time: Instant::now(),
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipeline,
            sun_render_pipeline,
            planets_texture_container,
            camera_container,
            instances,
            instance_buffer,
            depth_texture,
            sphere,
            sun,
            window,
        })
    }

    fn create_render_pipeline(
        device: &wgpu::Device,
        layout: &wgpu::PipelineLayout,
        color_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        vertex_layouts: &[wgpu::VertexBufferLayout],
        shader: wgpu::ShaderModuleDescriptor,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(shader);

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: vertex_layouts,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
                format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.camera_container.projection.resize(width, height);
        }
    }

    fn handle_key(
        &mut self,
        event_loop: &ActiveEventLoop,
        code: KeyCode,
        element_state: ElementState,
    ) {
        if code == KeyCode::Escape && element_state.is_pressed() {
            event_loop.exit();
        }
        self.camera_container
            .camera_controller
            .process_keyboard(code, element_state);
    }

    fn update(&mut self, dt: Duration) {
        self.camera_container.update(dt);
        self.queue.write_buffer(
            &self.camera_container.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_container.camera_uniform]),
        );
    }

    fn render(&mut self, dt: Duration) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        // Cannot renderd to not configured surface
        if !self.is_surface_configured {
            return Ok(());
        }

        self.update(dt);

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.01,
                        g: 0.01,
                        b: 0.01,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Render planets
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_texture_array_container(&self.planets_texture_container);
        render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
        render_pass.draw_sphere_instanced(
            &self.sphere,
            0..self.instances.len() as _,
            &self.camera_container.camera_bind_group,
            &self.sun.light.bind_group,
        );

        // Render sun
        render_pass.set_pipeline(&self.sun_render_pipeline);
        render_pass.set_texture_container(&self.sun.texture_container);
        render_pass.draw_sphere_instanced(
            &self.sphere,
            0..1,
            &self.camera_container.camera_bind_group,
            &self.sun.light.bind_group,
        );

        // `render_pass` mutably borrows encoder, so it must be dropped before using encoder again
        drop(render_pass);

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

pub struct App {
    /// We store state behind `Option` as `State` needs `Window`, but we get window only when
    /// app gets to `Reumed` state (look at [`ApplicationHandler`] implementation for [`App`])
    state: Option<State>,
}

impl App {
    pub fn new() -> Self {
        Self { state: None }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("Solar System");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            .unwrap();
        window.set_cursor_visible(false);
        self.state = Some(pollster::block_on(State::new(window)).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };

        let now = Instant::now();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let dt = now.duration_since(state.last_render_time);
                state.last_render_time = now;
                if let Err(e) = state.render(dt) {
                    match e {
                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                            let size = state.window.inner_size();
                            state.resize(size.width, size.height);
                        }
                        _ => {
                            log::error!("Unable to render: {e}");
                        }
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state),

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let state = match &mut self.state {
            Some(state) => state,
            None => return,
        };

        if let DeviceEvent::MouseMotion { delta } = event {
            state
                .camera_container
                .camera_controller
                .handle_mouse(delta.0, delta.1)
        }
    }
}

const NUM_INSTANCES_PER_ROW: u32 = 15;
const INSTANCE_DISPLACEMENT: glam::Vec3 = glam::Vec3::new(
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
    0.0,
    NUM_INSTANCES_PER_ROW as f32 * 0.5,
);
