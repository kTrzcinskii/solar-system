use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{DeviceEvent, ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{
    camera, hdr,
    planets::{self, DrawPlanets},
    skybox::{self, DrawSkybox},
    sphere,
    sun::{self, DrawSun},
    texture,
};

struct State {
    app_start_time: Instant,
    last_render_time: Instant,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    camera_container: camera::CameraContainer,
    depth_texture: texture::Texture,
    sphere: sphere::Sphere,
    sun: sun::Sun,
    planets: planets::Planets,
    hdr: hdr::HdrPipeline,
    skybox: skybox::Skybox,
    max_size: PhysicalSize<u32>,
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

        let max_dims = adapter.limits().max_texture_dimension_3d;
        let max_size = PhysicalSize::new(max_dims, max_dims);
        window.set_max_inner_size(Some(max_size));

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::all_webgpu_mask() & adapter.features(),
                required_limits: wgpu::Limits::downlevel_defaults(),
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
            view_formats: vec![surface_format.add_srgb_suffix()],
            desired_maximum_frame_latency: 2,
        };

        let hdr = hdr::HdrPipeline::new(&device, &config);

        let camera_container = camera::CameraContainer::new(config.width, config.height, &device);

        let skybox = skybox::Skybox::new(&device, &queue, &hdr, &camera_container)?;

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let sun = sun::Sun::new(&device, &queue, &hdr, &camera_container);

        let planets = planets::Planets::new(&device, &queue, &hdr, &camera_container, &sun);

        let sphere = sphere::Sphere::new(&device);

        let state = State {
            app_start_time: Instant::now(),
            last_render_time: Instant::now(),
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            camera_container,
            depth_texture,
            sphere,
            sun,
            planets,
            hdr,
            skybox,
            max_size,
            window,
        };
        state.update_window();
        Ok(state)
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
            self.hdr.resize(&self.device, width, height);
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
        if code == KeyCode::KeyL && element_state.is_pressed() {
            self.camera_container.camera_controller.swap_cursor_locked();
            self.update_window();
        }
        self.camera_container
            .camera_controller
            .process_keyboard(code, element_state);
    }

    fn update_window(&self) {
        match self.camera_container.camera_controller.cursor_locked() {
            true => {
                self.window
                    .set_cursor_grab(winit::window::CursorGrabMode::Confined)
                    .unwrap();
                self.window.set_cursor_visible(false);
            }
            false => {
                self.window
                    .set_cursor_grab(winit::window::CursorGrabMode::None)
                    .unwrap();
                self.window.set_cursor_visible(true);
            }
        }
    }

    fn update(&mut self, dt: Duration) {
        self.camera_container.update(dt);
        self.camera_container.sync_camera_buffer(&self.queue);
        self.planets.update(self.app_start_time.elapsed());
        self.planets.sync_instance_buffer(&self.queue);
        self.sun.update(self.app_start_time.elapsed());
        self.sun.sync_instance_buffer(&self.queue);
    }

    fn render(&mut self, dt: Duration) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        // Cannot renderd to not configured surface
        if !self.is_surface_configured {
            return Ok(());
        }

        self.update(dt);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: self.hdr.view(),
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

        render_pass.draw_planets(
            &self.planets,
            &self.sphere,
            &self.camera_container.camera_bind_group,
            &self.sun.light().bind_group,
        );

        render_pass.draw_sun(
            &self.sun,
            &self.sphere,
            &self.camera_container.camera_bind_group,
        );

        render_pass.draw_skybox(&self.skybox, &self.camera_container.camera_bind_group);

        // `render_pass` mutably borrows encoder, so it must be dropped before using encoder again
        drop(render_pass);

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.config.format.add_srgb_suffix()),
            ..Default::default()
        });

        // Apply tonemapping (HDR -> SDR)
        self.hdr.process(&mut encoder, &view);

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
            WindowEvent::Resized(size) => {
                if size.width > state.max_size.width || size.height > state.max_size.height {
                    let clamped_size = PhysicalSize::new(
                        size.width.min(state.max_size.width),
                        size.height.min(state.max_size.height),
                    );
                    state.resize(clamped_size.width, clamped_size.height);
                    let _ = state.window.request_inner_size(clamped_size);
                    state.window.request_redraw();
                    return;
                }
                state.resize(size.width, size.height);
            }
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
                .handle_mouse(delta.0, delta.1);
        }
    }
}
