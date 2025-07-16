use crate::{light, texture};

pub struct Sun {
    pub light: light::Light,
    pub texture_container: texture::TextureContainer,
}

impl Sun {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, position: [f32; 3]) -> Self {
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
