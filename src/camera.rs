use std::{f32::consts::FRAC_PI_2, time::Duration};

use wgpu::util::DeviceExt;
use winit::{event::ElementState, keyboard::KeyCode};

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug)]
pub struct Camera {
    pub position: glam::Vec3,
    yaw: f32,
    pitch: f32,
}

impl Camera {
    pub fn new<P: Into<glam::Vec3>>(position: P, yaw: f32, pitch: f32) -> Self {
        Self {
            position: position.into(),
            yaw,
            pitch,
        }
    }

    pub fn view_matrix(&self) -> glam::Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        glam::Mat4::look_to_rh(
            self.position,
            glam::Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw),
            glam::Vec3::Y,
        )
    }
}

pub struct Projection {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new(width: u32, height: u32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn projection_matrix(&self) -> glam::Mat4 {
        glam::Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_projection_matrix: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        CameraUniform {
            view_projection_matrix: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_projection_matrix(&mut self, camera: &Camera, projection: &Projection) {
        let view_matrix = camera.view_matrix();
        let projections_matrix = projection.projection_matrix();
        self.view_projection_matrix = (projections_matrix * view_matrix).to_cols_array_2d();
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    speed: f32,
    sensitivity: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn handle_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal += mouse_dx as f32;
        self.rotate_vertical += mouse_dy as f32;
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();

        // Calculate the forward vector based on yaw and pitch (look direction)
        let (sin_pitch, cos_pitch) = camera.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = camera.yaw.sin_cos();

        let forward =
            glam::Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize();

        // Right vector (perpendicular to forward and up)
        let right = glam::Vec3::new(-sin_yaw, 0.0, cos_yaw).normalize();

        // Up vector (world up)
        let up = glam::Vec3::Y;

        // Move in the direction the camera is facing
        camera.position += forward * (self.amount_forward - self.amount_backward) * self.speed * dt;
        camera.position += right * (self.amount_right - self.amount_left) * self.speed * dt;
        camera.position += up * (self.amount_up - self.amount_down) * self.speed * dt;

        // Rotate
        camera.yaw += (self.rotate_horizontal).to_radians() * self.sensitivity * dt;
        camera.pitch += (-self.rotate_vertical).to_radians() * self.sensitivity * dt;

        self.rotate_horizontal = 0.0;
        self.rotate_vertical = 0.0;

        // Clamp pitch
        camera.pitch = camera.pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
    }
}

pub struct CameraContainer {
    pub camera: Camera,
    pub projection: Projection,
    pub camera_controller: CameraController,
    pub camera_bind_group: wgpu::BindGroup,
    pub camera_bind_group_layout: wgpu::BindGroupLayout,
    pub camera_buffer: wgpu::Buffer,
    pub camera_uniform: CameraUniform,
}

impl CameraContainer {
    pub fn new(width: u32, height: u32, device: &wgpu::Device) -> Self {
        let camera = Camera::new(
            (0.0, 5.0, 10.0),
            (-90.0_f32).to_radians(),
            (-20.0_f32).to_radians(),
        );
        let projection = Projection::new(width, height, (45.0_f32).to_radians(), 0.1, 100.0);
        let camera_controller = CameraController::new(4.0, 12.0);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_projection_matrix(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            camera,
            projection,
            camera_controller,
            camera_bind_group,
            camera_bind_group_layout,
            camera_buffer,
            camera_uniform,
        }
    }

    pub fn update(&mut self, dt: Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform
            .update_view_projection_matrix(&self.camera, &self.projection);
    }
}
