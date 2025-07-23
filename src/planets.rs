use std::time::Duration;

use wgpu::util::DeviceExt;

use crate::{
    instance::{self, Instance},
    sphere::{DrawSphere, Sphere},
    texture::{self, SetTextureContainer},
};

pub struct Planets {
    pub instances: Vec<instance::Instance>,
    pub instance_buffer: wgpu::Buffer,
    pub texture_container: texture::TextureContainer,
}

impl Planets {
    const PLANETS_COUNT: usize = 8;

    const PLANETS_RADIUS: [f32; Self::PLANETS_COUNT] =
        [12.5, 17.5, 25.0, 32.5, 42.5, 55.0, 65.0, 77.5];

    const PLANETS_SCALE: [f32; Self::PLANETS_COUNT] = [0.5, 0.7, 1.3, 1.0, 3.0, 2.5, 1.8, 1.8];

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let instances = (0..Self::PLANETS_COUNT)
            .map(|i| {
                let position = glam::Vec3::new(Self::PLANETS_RADIUS[i], 0.0, 0.0);
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

        Planets {
            instances,
            instance_buffer,
            texture_container,
        }
    }

    pub fn update(&mut self, total_time: Duration) {
        // TODO: add rotation around itself
        let t = total_time.as_secs_f32();
        for (i, instance) in self.instances.iter_mut().enumerate() {
            let radius = Self::PLANETS_RADIUS[i];
            let i = i as f32;
            let speed = 0.2 - 0.02 * i - 0.0002 * i * i;
            let angle = t * speed;
            instance.position = glam::Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin());
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
