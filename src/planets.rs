use core::f32;
use std::time::Duration;

use wgpu::util::DeviceExt;

use crate::{
    camera, hdr,
    instance::{self, Instance},
    pipeline,
    sphere::{self, DrawSphere, Sphere, Vertex},
    sun,
    texture::{self, SetTextureContainer},
};

pub struct Planets {
    instances: Vec<instance::Instance>,
    instance_buffer: wgpu::Buffer,
    texture_container: texture::TextureContainer,
    render_pipeline: wgpu::RenderPipeline,
}

impl Planets {
    const PLANETS_COUNT: usize = 8;

    const PLANETS_RADIUS: [f32; Self::PLANETS_COUNT] =
        [12.5, 17.5, 25.0, 32.5, 42.5, 55.0, 65.0, 77.5];

    const PLANETS_SCALE: [f32; Self::PLANETS_COUNT] = [0.5, 0.7, 1.3, 1.0, 3.0, 2.5, 1.8, 1.8];

    const INITIAL_OFFSET: [f32; Self::PLANETS_COUNT] = [
        f32::consts::FRAC_PI_4 * 3.0,
        f32::consts::FRAC_PI_4 * 7.0,
        f32::consts::PI * 2.0,
        f32::consts::FRAC_PI_2 * 3.0,
        f32::consts::FRAC_PI_2,
        f32::consts::FRAC_PI_4 * 5.0,
        f32::consts::PI,
        f32::consts::FRAC_PI_4,
    ];

    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hdr: &hdr::HdrPipeline,
        camera_container: &camera::CameraContainer,
        sun: &sun::Sun,
    ) -> Self {
        let instances = (0..Self::PLANETS_COUNT)
            .map(|i| {
                let initial_offset = Self::INITIAL_OFFSET[i];
                let radius = Self::PLANETS_RADIUS[i];
                let position = glam::Vec3::new(
                    radius * initial_offset.cos(),
                    0.0,
                    radius * initial_offset.sin(),
                );
                let rotation = glam::Quat::from_axis_angle(
                    position.normalize(),
                    (5.0 * i as f32).to_radians(),
                );
                Instance::new(position, rotation, i as _, Self::PLANETS_SCALE[i])
            })
            .collect::<Vec<_>>();

        let instance_data = instances
            .iter()
            .map(instance::InstanceRaw::from)
            .collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let texture_container =
            texture::TextureContainer::initialize_plantes_texture_array_container(device, queue);

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_container.bind_group_layout,
                    &camera_container.camera_bind_group_layout,
                    &sun.light().bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let shader = wgpu::include_wgsl!("../shaders/planet.wgsl");
        let render_pipeline = pipeline::create_render_pipeline(
            device,
            &render_pipeline_layout,
            hdr.format(),
            Some(texture::Texture::DEPTH_FORMAT),
            &[sphere::SphereVertex::desc(), instance::InstanceRaw::desc()],
            wgpu::PrimitiveTopology::TriangleList,
            shader,
            Some("render_pipeline_planets"),
        );

        Planets {
            instances,
            instance_buffer,
            texture_container,
            render_pipeline,
        }
    }

    pub fn update(&mut self, total_time: Duration) {
        let t = total_time.as_secs_f32();
        for (i, instance) in self.instances.iter_mut().enumerate() {
            let radius = Self::PLANETS_RADIUS[i];
            let offset = Self::INITIAL_OFFSET[i];
            let i = i as f32;

            let movement_speed = 0.15 - 0.015 * i - 0.0002 * i * i;
            let movement_angle = t * movement_speed + offset;
            instance.position = glam::Vec3::new(
                radius * movement_angle.cos(),
                0.0,
                radius * movement_angle.sin(),
            );

            let rotation_speed = 0.5 - 0.05 * i;
            let rotation_angle = t * rotation_speed;
            instance.rotation = glam::Quat::from_rotation_y(rotation_angle);
        }
    }

    pub fn sync_instance_buffer(&self, queue: &wgpu::Queue) {
        let instance_data = self
            .instances
            .iter()
            .map(instance::InstanceRaw::from)
            .collect::<Vec<_>>();
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }
}

pub trait DrawPlanets<'a> {
    fn draw_planets(
        &mut self,
        planets: &'a Planets,
        sphere: &'a Sphere,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawPlanets<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_planets(
        &mut self,
        planets: &'b Planets,
        sphere: &'b Sphere,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_pipeline(&planets.render_pipeline);
        self.set_texture_array_container(&planets.texture_container);
        self.set_vertex_buffer(1, planets.instance_buffer.slice(..));
        self.draw_sphere_instanced(
            sphere,
            0..planets.instances.len() as _,
            camera_bind_group,
            light_bind_group,
        );
    }
}
