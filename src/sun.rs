use crate::{
    light,
    sphere::{DrawSphere, Sphere},
    texture::{self, SetTextureContainer},
};

pub struct Sun {
    pub light: light::Light,
    pub texture_container: texture::TextureContainer,
}

impl Sun {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let position = [0.0, 0.0, 0.0];
        let light = light::Light::new(device, position, [1.0, 1.0, 1.0]);

        let texture_bytes = include_bytes!("../assets/textures/sun.jpg");
        let texture =
            texture::Texture::from_bytes(device, queue, texture_bytes, "sun texture").unwrap();
        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let texture_bind_group: wgpu::BindGroup =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture.sampler),
                    },
                ],
                label: Some("diffuse_bind_group"),
            });
        let texture_container =
            texture::TextureContainer::new(texture, texture_bind_group, texture_bind_group_layout);

        Self {
            light,
            texture_container,
        }
    }
}

pub trait DrawSun<'a> {
    fn draw_sun(
        &mut self,
        sun: &'a Sun,
        sphere: &'a Sphere,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawSun<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_sun(
        &mut self,
        sun: &'a Sun,
        sphere: &'a Sphere,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_texture_container(&sun.texture_container);
        self.draw_sphere_instanced(sphere, 0..1, camera_bind_group, &sun.light.bind_group);
    }
}
