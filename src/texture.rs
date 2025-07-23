use anyhow::Result;
use image::GenericImageView;

pub struct Texture {
    pub _texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            _texture: texture,
            view,
            sampler,
        })
    }

    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }

    /// IMPORTANT NOTE: each texture used in this array must be of same size
    pub fn create_texture_array(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        image_paths: &[&str],
        label: &str,
    ) -> Self {
        let images: Vec<_> = image_paths
            .iter()
            .map(|path| image::open(path).expect("Failed to load image"))
            .collect();

        let (width, height) = images[0].dimensions();
        let layer_count = images.len() as u32;

        let mut texture_data = Vec::with_capacity((width * height * 4 * layer_count) as usize);
        for img in &images {
            let rgba = img.to_rgba8();
            texture_data.extend_from_slice(&rgba);
        }

        let texture_size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: layer_count,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &texture_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            texture_size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            _texture: texture,
            view,
            sampler,
        }
    }
}

pub struct TextureContainer {
    _texture: Texture,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl TextureContainer {
    const TEXTURE_BIND_GROUP_ID: u32 = 0;
    const TEXTURE_ARRAY_CONTAINER_ID: u32 = 0;

    pub fn new(
        texture: Texture,
        bind_group: wgpu::BindGroup,
        bind_group_layout: wgpu::BindGroupLayout,
    ) -> Self {
        Self {
            _texture: texture,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn initialize_plantes_texture_array_container(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let diffuse_texture = Texture::create_texture_array(
            device,
            queue,
            &[
                "./assets/textures/mercury.jpg",
                "./assets/textures/venus.jpg",
                "./assets/textures/earth.jpg",
                "./assets/textures/earth_night.jpg",
                "./assets/textures/mars.jpg",
                "./assets/textures/jupiter.jpg",
                "./assets/textures/saturn.jpg",
                "./assets/textures/uranus.jpg",
                "./assets/textures/neptune.jpg",
            ],
            "planets-textures",
        );

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_array_bind_group_layout"),
            });

        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: Some("texture_array_bind_group"),
        });

        Self::new(
            diffuse_texture,
            diffuse_bind_group,
            texture_bind_group_layout,
        )
    }
}

pub trait SetTextureContainer<'a> {
    fn set_texture_container(&mut self, texture_container: &'a TextureContainer);
    fn set_texture_array_container(&mut self, texture_container: &'a TextureContainer);
}

impl<'a, 'b> SetTextureContainer<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn set_texture_container(&mut self, texture_container: &'b TextureContainer) {
        self.set_bind_group(
            TextureContainer::TEXTURE_BIND_GROUP_ID,
            &texture_container.bind_group,
            &[],
        );
    }
    fn set_texture_array_container(&mut self, texture_container: &'b TextureContainer) {
        self.set_bind_group(
            TextureContainer::TEXTURE_ARRAY_CONTAINER_ID,
            &texture_container.bind_group,
            &[],
        );
    }
}
