use std::time::Duration;

use wgpu::util::DeviceExt;

use crate::{
    camera, hdr, instance, light, pipeline,
    sphere::{self, DrawSphere, Sphere},
    texture::{self, SetTextureContainer},
    vertex::Vertex,
};

pub struct Sun {
    light: light::Light,
    instance: instance::Instance,
    instance_buffer: wgpu::Buffer,
    texture_container: texture::TextureContainer,
    render_pipeline: wgpu::RenderPipeline,
}

impl Sun {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        hdr: &hdr::HdrPipeline,
        camera_container: &camera::CameraContainer,
    ) -> Self {
        let position = [0.0, 0.0, 0.0];
        let light = light::Light::new(device, position, [1.0, 1.0, 1.0]);

        let instance =
            instance::Instance::new(position.into(), glam::Quat::from_rotation_y(0.0), 0, 6.5);

        let instance_data = vec![instance::InstanceRaw::from(&instance)];
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sun Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

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

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sun Pipeline Layout"),
            bind_group_layouts: &[
                &texture_container.bind_group_layout,
                &camera_container.camera_bind_group_layout,
                &light.bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let shader = wgpu::include_wgsl!("../shaders/sun.wgsl");
        let render_pipeline = pipeline::create_render_pipeline(
            device,
            &layout,
            hdr.format(),
            Some(texture::Texture::DEPTH_FORMAT),
            &[sphere::SphereVertex::desc(), instance::InstanceRaw::desc()],
            wgpu::PrimitiveTopology::TriangleList,
            shader,
            Some("render_pipelie_sun"),
        );

        Self {
            light,
            instance,
            instance_buffer,
            texture_container,
            render_pipeline,
        }
    }

    pub fn light(&self) -> &light::Light {
        &self.light
    }

    pub fn update(&mut self, total_time: Duration) {
        let t = total_time.as_secs_f32();
        let rotation_speed = 0.12;
        let rotation_angle = t * rotation_speed;
        self.instance.rotation = glam::Quat::from_rotation_y(rotation_angle);
    }

    pub fn sync_instance_buffer(&self, queue: &wgpu::Queue) {
        let instance_data = vec![instance::InstanceRaw::from(&self.instance)];
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
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
        self.set_pipeline(&sun.render_pipeline);
        self.set_texture_container(&sun.texture_container);
        self.set_vertex_buffer(1, sun.instance_buffer.slice(..));
        self.draw_sphere_instanced(sphere, 0..1, camera_bind_group, &sun.light.bind_group);
    }
}
