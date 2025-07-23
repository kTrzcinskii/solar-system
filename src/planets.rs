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

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let instances = (0..Self::PLANETS_COUNT)
            .map(|i| {
                let position = glam::Vec3::new(5.0 * (i + 1) as f32, 0.0, 12.0);
                let rotation = glam::Quat::from_axis_angle(
                    position.normalize(),
                    (5.0 * i as f32).to_radians(),
                );
                Instance::new(position, rotation, i as _)
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

        let texture_container =
            texture::TextureContainer::initialize_plantes_texture_array_container(device, queue);

        Planets {
            instances,
            instance_buffer,
            texture_container,
        }
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
