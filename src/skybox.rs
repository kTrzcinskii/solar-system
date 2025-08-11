use anyhow::Result;

use crate::{hdr, texture};

pub struct Skybox {
    _cubemap: texture::CubeTexture,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl Skybox {
    const DST_SIZE: u32 = 1080;

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Self> {
        let hdr_loader = hdr::HdrLoader::new(device);
        let skybox_bytes = include_bytes!("../assets/textures/stars.jpg");
        let skybox_texture = hdr_loader.equirectangular_bytes(
            device,
            queue,
            skybox_bytes,
            Self::DST_SIZE,
            Some("Skybox"),
        )?;

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("environment_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::Cube,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("environment_bind_group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(skybox_texture.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(skybox_texture.sampler()),
                },
            ],
        });

        Ok(Skybox {
            _cubemap: skybox_texture,
            bind_group_layout: layout,
            bind_group,
        })
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
