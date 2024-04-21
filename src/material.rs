use crate::{state::State, utils::noise::perlin_noise};
use image::GenericImageView;

impl Texture {
    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
    pub fn create_depth_texture(state: &State) -> Self {
        let size = wgpu::Extent3d {
            width: state.surface_config.width,
            height: state.surface_config.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("depth_texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = state.device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual), // 5.
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });
        Self {
            data: None,
            texture,
            view,
            sampler,
            name: String::from("depth_texture"),
        }
    }
    //
    pub fn create_perlin_noise_texture(
        width: u32,
        height: u32,
        frequency: f32,
        state: &State,
    ) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = state.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("perlin_noise"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: &[],
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            format: wgpu::TextureFormat::Rgba8Unorm,
        });
        let mut perlin_noise_data: Vec<f32> = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                perlin_noise_data.push(perlin_noise(
                    x as f32 * frequency,
                    y as f32 * frequency,
                    (width as f32 * frequency) as u32,
                ))
            }
        }

        let data: Vec<_> = perlin_noise_data
            .iter()
            .flat_map(|v| {
                let u = f32::round((v + 1.0) * 0.5 * 255.0) as u8;
                [u, u, u, 255]
            })
            .collect();

        state.queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            data.as_slice(),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
            ..Default::default()
        });

        Self {
            view,
            sampler,
            texture,
            name: "perlin_noise".to_string(),
            data: Some(data),
        }
    }

    pub fn from_bytes(
        bytes: &[u8],
        name: String,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let image = image::load_from_memory(bytes)?;
        let dimensions = image.dimensions();
        let rgba = image.as_rgba8().unwrap();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&name.clone()),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: &[],
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            ..Default::default()
        });

        Ok(Self {
            view,
            sampler,
            texture,
            name,
            data: None,
        })
    }
    pub fn from_path(
        path: &str,
        name: String,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let f = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(f);
        let image = image::load(reader, image::ImageFormat::Png)?;
        let dimensions = image.dimensions();
        let rgba = image.as_rgba8().unwrap();

        let size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&name.clone()),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: &[],
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            ..Default::default()
        });

        Ok(Self {
            view,
            sampler,
            texture,
            name,
            data: None,
        })
    }
}

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub name: String,
    pub data: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct Material {
    pub diffuse: Texture,
}
