use anyhow::Result;

use crate::{camera, hdr, pipeline, texture};

pub struct Skybox {
    _cubemap: texture::CubeTexture,
    bind_group: wgpu::BindGroup,
    render_pipeline: wgpu::RenderPipeline,
}

impl Skybox {
    const DST_SIZE: u32 = 1080;

    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hdr: &hdr::HdrPipeline,
        camera_container: &camera::CameraContainer,
    ) -> Result<Self> {
        let hdr_loader = hdr::HdrLoader::new(device);
        let skybox_bytes = include_bytes!("../assets/textures/stars.jpg");
        let skybox_texture = hdr_loader.equirectangular_bytes(
            device,
            queue,
            skybox_bytes,
            Self::DST_SIZE,
            Some("Skybox"),
        )?;

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            layout: &bind_group_layout,
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

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Sky Pipeline Layout"),
                bind_group_layouts: &[
                    &camera_container.camera_bind_group_layout,
                    &bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let shader = wgpu::include_wgsl!("../shaders/skybox.wgsl");
        let render_pipeline = pipeline::create_render_pipeline(
            device,
            &render_pipeline_layout,
            hdr.format(),
            Some(texture::Texture::DEPTH_FORMAT),
            &[],
            wgpu::PrimitiveTopology::TriangleList,
            shader,
            Some("render_pipeline_skybox"),
        );

        Ok(Skybox {
            _cubemap: skybox_texture,
            bind_group,
            render_pipeline,
        })
    }
}

pub trait DrawSkybox<'a> {
    fn draw_skybox(&mut self, skybox: &'a Skybox, camera_bind_group: &'a wgpu::BindGroup);
}

impl<'a, 'b> DrawSkybox<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_skybox(&mut self, skybox: &'a Skybox, camera_bind_group: &'a wgpu::BindGroup) {
        self.set_pipeline(&skybox.render_pipeline);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, &skybox.bind_group, &[]);
        self.draw(0..3, 0..1);
    }
}
