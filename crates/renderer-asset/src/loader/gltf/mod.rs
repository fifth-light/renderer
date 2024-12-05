use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    io::{self, Cursor},
    iter,
    marker::PhantomData,
    sync::Arc,
};

use glam::{Mat4, Quat, Vec3};
use gltf::{
    accessor::{DataType, Dimensions},
    animation::{Channel, Interpolation, Property, Sampler},
    camera::Projection,
    image::Format,
    json::Value,
    material::AlphaMode,
    mesh::{Mode, MorphTarget},
    scene::Transform,
    texture::{self, MagFilter, MinFilter, WrappingMode},
    Accessor, Animation, Camera, Document, Gltf, Material, Mesh, Node, Primitive, Scene, Semantic,
    Skin, Texture,
};
use image::{guess_format, DynamicImage, GenericImageView, ImageError, ImageFormat, ImageReader};
use scheme::{Scheme, SchemeError};

use crate::{
    animation::{
        AnimationAsset, AnimationChannelAsset, AnimationKeyFrame, AnimationKeyFrames,
        AnimationSampler,
    },
    archive::{Archive, Entry},
    camera::{CameraAsset, CameraProjectionAsset, OrthographicCameraAsset, PerspectiveCameraAsset},
    index::{AssetIndex, BundleAssetType, BundleIndex},
    loader::{
        chunk_and_clamp_vec3_to_vec4_f32, chunk_and_clamp_vec4_f32, chunk_vec3, chunk_vec4,
        clip_vec4_to_vec3, pad_vec3_to_vec4,
    },
    material::{
        MaterialAlphaMode, MaterialAsset, MaterialAssetData, OutlineWidthMode, UvAnimation,
    },
    mesh::MeshAsset,
    node::{DecomposedTransform, MatrixNodeTransform, NodeAsset, NodeTransform},
    primitive::{
        Joints, Normal, Position, PrimitiveAsset, PrimitiveAssetAttributes, PrimitiveAssetMode,
        PrimitiveAssetMorphTarget, Tangent, TexCoord, VertexColor, Weights,
    },
    scene::SceneAsset,
    skin::SkinAsset,
    tangent::calculate_tangent,
    texture::{
        NormalTextureInfo, OcclusionTextureInfo, SamplerAsset, ShadingShiftTextureInfo,
        TextureAsset, TextureAssetFormat, TextureAssetTransform, TextureInfo, TextureMagFilter,
        TextureMinFilter, TextureMipmapFilter, TextureWrappingMode,
    },
};

use super::{chunk_mat4, AssetLoadParams};

pub mod scheme;

#[derive(Debug, Clone)]
pub enum GltfImageSource {
    Buffer(usize),
    Uri(String),
}

impl Display for GltfImageSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GltfImageSource::Buffer(index) => write!(f, "buffer #{}", index),
            GltfImageSource::Uri(uri) => Display::fmt(uri, f),
        }
    }
}

#[derive(Debug)]
pub enum GltfLoaderError<E> {
    Gltf(gltf::Error),
    Io(E),
    ModelNotFound(String),
    BadModelFile,
    InvalidScheme(SchemeError),
    ResourceNotFound(String),
    BadBufferMime(String, Option<String>),
    BadImage(GltfImageSource, ImageError),
    BadImageFormat(GltfImageSource),
    BadImageMime(GltfImageSource, String),
    ImageBufferOutOfBounds(usize, usize, usize),
    UnsupportedTextureFormat(Format),
    BadMToonData,
    UnsupportedPrimitiveMode(gltf::mesh::Mode),
    BadAccessorDataType(DataType, DataType),
    BadAccessorDimensions(Dimensions, Dimensions),
    BadJointWeightSets(usize, usize),
}

impl<E: Display> Display for GltfLoaderError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GltfLoaderError::Gltf(error) => Display::fmt(error, f),
            GltfLoaderError::Io(error) => Display::fmt(error, f),
            GltfLoaderError::ModelNotFound(file_name) => {
                write!(f, "File {} not found in bundle", file_name)
            }
            GltfLoaderError::BadModelFile => write!(f, "Bad model file"),
            GltfLoaderError::InvalidScheme(error) => Display::fmt(error, f),
            GltfLoaderError::ResourceNotFound(name) => write!(f, "Resource {} not found", name),
            GltfLoaderError::BadBufferMime(name, mime) => {
                if let Some(mime) = mime {
                    write!(f, "Bad MIME {} for buffer {}", mime, name)
                } else {
                    write!(f, "No MIME for buffer {}", name)
                }
            }
            GltfLoaderError::BadImage(name, error) => write!(f, "Bad image {}: {}", name, error),
            GltfLoaderError::BadImageMime(name, mime) => {
                write!(f, "Bad MIME {} for image {}", mime, name)
            }
            GltfLoaderError::BadImageFormat(name) => {
                write!(f, "Bad format for image {}", name)
            }
            GltfLoaderError::ImageBufferOutOfBounds(index, buffer_index, buffer_length) => write!(
                f,
                "Buffer index of bounds of image #{}: {} of {}",
                index, buffer_index, buffer_length
            ),
            GltfLoaderError::UnsupportedTextureFormat(format) => {
                write!(f, "Unsupported texture format: {:?}", format)
            }
            GltfLoaderError::BadMToonData => write!(f, "Bad MToon data"),
            GltfLoaderError::UnsupportedPrimitiveMode(mode) => {
                write!(f, "Unsupported primitive mode: {:?}", mode)
            }
            GltfLoaderError::BadAccessorDataType(expected, actual) => {
                write!(
                    f,
                    "Bad accessor data type: expected {:?}, but got {:?}",
                    expected, actual
                )
            }
            GltfLoaderError::BadAccessorDimensions(expected, actual) => {
                write!(
                    f,
                    "Bad accessor dimensions: expected {:?}, but got {:?}",
                    expected, actual
                )
            }
            GltfLoaderError::BadJointWeightSets(joints, weights) => {
                write!(
                    f,
                    "Unpaired joints and weights: joints {}, weights {}",
                    joints, weights
                )
            }
        }
    }
}

impl<E> From<gltf::Error> for GltfLoaderError<E> {
    fn from(value: gltf::Error) -> Self {
        Self::Gltf(value)
    }
}

impl<E> From<SchemeError> for GltfLoaderError<E> {
    fn from(value: SchemeError) -> Self {
        Self::InvalidScheme(value)
    }
}

impl<E: Error> Error for GltfLoaderError<E> {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AnimationPathType {
    Rotation,
    Translation,
    Scale,
}

#[derive(Debug)]
struct GltfData {
    bundle_index: BundleIndex,
    buffers: Vec<gltf::buffer::Data>,
    images: Vec<gltf::image::Data>,
}

struct GltfDocumentLoader<'a, E> {
    document: &'a Document,
    data: &'a GltfData,
    params: &'a AssetLoadParams,
    texture_cache: HashMap<usize, Arc<TextureAsset>>,
    material_cache: HashMap<usize, Arc<MaterialAsset>>,
    skin_cache: HashMap<usize, Arc<SkinAsset>>,
    _markor: PhantomData<E>,
}

type GLTFLoadResult<E> = Result<(Vec<SceneAsset>, Vec<AnimationAsset>), GltfLoaderError<E>>;

impl<'a, E> GltfDocumentLoader<'a, E> {
    fn new(document: &'a Document, data: &'a GltfData, params: &'a AssetLoadParams) -> Self {
        Self {
            document,
            data,
            params,
            texture_cache: HashMap::new(),
            material_cache: HashMap::new(),
            skin_cache: HashMap::new(),
            _markor: PhantomData,
        }
    }

    #[inline]
    fn check_accessor(
        accessor: &Accessor,
        data_type: DataType,
        dimensions: Dimensions,
    ) -> Result<(), GltfLoaderError<E>> {
        let actual_data_type = accessor.data_type();
        if actual_data_type != data_type {
            return Err(GltfLoaderError::BadAccessorDataType(
                data_type,
                actual_data_type,
            ));
        }

        let actual_dimensions = accessor.dimensions();
        if actual_dimensions != dimensions {
            return Err(GltfLoaderError::BadAccessorDimensions(
                dimensions,
                actual_dimensions,
            ));
        }

        Ok(())
    }

    #[inline]
    fn load_accessor(data: &GltfData, accessor: &Accessor) -> Vec<u8> {
        let view = if let Some(view) = accessor.view() {
            view
        } else {
            return iter::repeat(0).take(accessor.count()).collect();
        };
        let num_size: usize = match accessor.data_type() {
            DataType::I8 => 1,
            DataType::U8 => 1,
            DataType::I16 => 2,
            DataType::U16 => 2,
            DataType::U32 => 4,
            DataType::F32 => 4,
        };
        let item_size: usize = match accessor.dimensions() {
            Dimensions::Scalar => 1,
            Dimensions::Vec2 => 2,
            Dimensions::Vec3 => 3,
            Dimensions::Vec4 => 4,
            Dimensions::Mat2 => 4,
            Dimensions::Mat3 => 9,
            Dimensions::Mat4 => 16,
        };
        let item_length: usize = num_size * item_size;

        let count = accessor.count();
        let buffer = &data.buffers[view.buffer().index()];
        let offset = accessor.offset() + view.offset();
        let stride = view.stride().unwrap_or(item_length);
        let interval = stride - item_length;
        let full_length = count * (item_length + interval);

        assert!(offset + full_length <= buffer.len());

        if stride == 0 {
            (offset..offset + full_length)
                .map(|index| buffer.0[index])
                .collect()
        } else {
            let read_length = item_length * count;
            let mut result = Vec::new();
            let mut chunk_left = item_length;
            let mut index = offset;
            let mut read_amount = 0;
            while read_amount < read_length {
                result.push(buffer.0[index]);
                read_amount += 1;
                index += 1;
                chunk_left -= 1;
                if chunk_left == 0 {
                    index += interval;
                    chunk_left = item_length;
                }
            }
            result
        }
    }

    #[inline]
    fn load_accessor_f32(data: &GltfData, accessor: &Accessor) -> Vec<f32> {
        assert_eq!(accessor.data_type(), DataType::F32);
        let data = Self::load_accessor(data, accessor);
        data.chunks_exact(4)
            .map(|chunk| {
                let array = chunk.try_into().unwrap();
                f32::from_le_bytes(array)
            })
            .collect()
    }

    #[inline]
    fn load_accessor_u8(data: &GltfData, accessor: &Accessor) -> Vec<u8> {
        assert_eq!(accessor.data_type(), DataType::U8);
        Self::load_accessor(data, accessor)
    }

    #[inline]
    fn normalize_u8(data: Vec<u8>) -> Vec<f32> {
        data.into_iter()
            .map(|item| item as f32 / u8::MAX as f32)
            .collect()
    }

    #[inline]
    fn load_accessor_u16(data: &GltfData, accessor: &Accessor) -> Vec<u16> {
        assert_eq!(accessor.data_type(), DataType::U16);
        let data = Self::load_accessor(data, accessor);
        data.chunks_exact(2)
            .map(|chunk| {
                let array = chunk.try_into().unwrap();
                u16::from_le_bytes(array)
            })
            .collect()
    }

    #[inline]
    fn normalize_u16(data: Vec<u16>) -> Vec<f32> {
        data.into_iter()
            .map(|item| item as f32 / u16::MAX as f32)
            .collect()
    }

    #[inline]
    fn load_accessor_u32(data: &GltfData, accessor: &Accessor) -> Vec<u32> {
        assert_eq!(accessor.data_type(), DataType::U32);
        let data = Self::load_accessor(data, accessor);
        data.chunks_exact(4)
            .map(|chunk| {
                let array = chunk.try_into().unwrap();
                u32::from_le_bytes(array)
            })
            .collect()
    }

    #[inline]
    fn normalize_u32(data: Vec<u32>) -> Vec<f32> {
        data.into_iter()
            .map(|item| item as f32 / u32::MAX as f32)
            .collect()
    }

    #[inline]
    fn load_accessor_normalized(data: &GltfData, accessor: &Accessor) -> Vec<f32> {
        match accessor.data_type() {
            DataType::U8 => Self::normalize_u8(Self::load_accessor_u8(data, accessor)),
            DataType::U16 => Self::normalize_u16(Self::load_accessor_u16(data, accessor)),
            DataType::U32 => Self::normalize_u32(Self::load_accessor_u32(data, accessor)),
            DataType::F32 => Self::load_accessor_f32(data, accessor),
            _ => unreachable!("Invalid normalized data"),
        }
    }

    fn load_vec2_value(value: &Value) -> Result<[f32; 2], GltfLoaderError<E>> {
        let array = value.as_array().ok_or(GltfLoaderError::BadMToonData)?;
        if array.len() == 3 {
            let x = array[0].as_f64().ok_or(GltfLoaderError::BadMToonData)? as f32;
            let y = array[1].as_f64().ok_or(GltfLoaderError::BadMToonData)? as f32;
            Ok([x, y])
        } else {
            Err(GltfLoaderError::BadMToonData)
        }
    }

    fn load_vec3_value(value: &Value) -> Result<[f32; 3], GltfLoaderError<E>> {
        let array = value.as_array().ok_or(GltfLoaderError::BadMToonData)?;
        if array.len() == 3 {
            let r = array[0].as_f64().ok_or(GltfLoaderError::BadMToonData)? as f32;
            let g = array[1].as_f64().ok_or(GltfLoaderError::BadMToonData)? as f32;
            let b = array[2].as_f64().ok_or(GltfLoaderError::BadMToonData)? as f32;
            Ok([r, g, b])
        } else {
            Err(GltfLoaderError::BadMToonData)
        }
    }

    fn load_texture_sampler(sampler: gltf::texture::Sampler) -> SamplerAsset {
        let (min_filter, mipmap_filter) = sampler
            .min_filter()
            .map(|filter| match filter {
                MinFilter::Nearest => (TextureMinFilter::Nearest, TextureMipmapFilter::default()),
                MinFilter::Linear => (TextureMinFilter::Linear, TextureMipmapFilter::default()),
                MinFilter::NearestMipmapNearest => {
                    (TextureMinFilter::Nearest, TextureMipmapFilter::Nearest)
                }
                MinFilter::LinearMipmapNearest => {
                    (TextureMinFilter::Linear, TextureMipmapFilter::Nearest)
                }
                MinFilter::NearestMipmapLinear => {
                    (TextureMinFilter::Nearest, TextureMipmapFilter::Linear)
                }
                MinFilter::LinearMipmapLinear => {
                    (TextureMinFilter::Linear, TextureMipmapFilter::Linear)
                }
            })
            .unwrap_or_default();

        fn wrapping_mode(mode: WrappingMode) -> TextureWrappingMode {
            match mode {
                WrappingMode::ClampToEdge => TextureWrappingMode::ClampToEdge,
                WrappingMode::MirroredRepeat => TextureWrappingMode::MirroredRepeat,
                WrappingMode::Repeat => TextureWrappingMode::Repeat,
            }
        }

        SamplerAsset {
            mag_filter: sampler
                .mag_filter()
                .map(|filter| match filter {
                    MagFilter::Nearest => TextureMagFilter::Nearest,
                    MagFilter::Linear => TextureMagFilter::Linear,
                })
                .unwrap_or_default(),
            min_filter,
            mipmap_filter,
            wrap_x: wrapping_mode(sampler.wrap_s()),
            wrap_y: wrapping_mode(sampler.wrap_t()),
        }
    }

    fn load_camera(camera: Camera) -> CameraAsset {
        let data = match camera.projection() {
            Projection::Orthographic(orthographic) => {
                CameraProjectionAsset::Orthographic(OrthographicCameraAsset {
                    xmag: orthographic.xmag(),
                    ymag: orthographic.ymag(),
                    zfar: orthographic.zfar(),
                    znear: orthographic.znear(),
                })
            }
            Projection::Perspective(perspective) => {
                CameraProjectionAsset::Perspective(PerspectiveCameraAsset {
                    aspect_radio: perspective.aspect_ratio(),
                    yfov: perspective.yfov(),
                    zfar: perspective.zfar(),
                    znear: perspective.znear(),
                })
            }
        };
        CameraAsset {
            name: camera.name().map(str::to_string),
            projection: data,
        }
    }

    fn load_texture(&mut self, texture: Texture) -> Result<Arc<TextureAsset>, GltfLoaderError<E>> {
        let sampler = Self::load_texture_sampler(texture.sampler());
        let image = texture.source();
        let index: usize = image.index();
        let id = AssetIndex::BundleTypeIndex(
            self.data.bundle_index.clone(),
            BundleAssetType::Texture,
            index,
        );

        if let Some(asset) = self.texture_cache.get(&index) {
            return Ok(asset.clone());
        }

        let image = &self.data.images[index];
        let format = match image.format {
            Format::R8 => TextureAssetFormat::Ru8,
            Format::R8G8 => TextureAssetFormat::Rgu8,
            Format::R8G8B8 => TextureAssetFormat::Rgbu8,
            Format::R8G8B8A8 => TextureAssetFormat::Rgbau8,
            Format::R16 => TextureAssetFormat::Ru16,
            Format::R16G16 => TextureAssetFormat::Rgu16,
            Format::R16G16B16 => TextureAssetFormat::Rgbu16,
            Format::R16G16B16A16 => TextureAssetFormat::Rgbau16,
            _ => return Err(GltfLoaderError::UnsupportedTextureFormat(image.format)),
        };
        let asset = Arc::new(TextureAsset {
            id,
            size: (image.width, image.height),
            format,
            data: image.pixels.clone(),
            sampler,
        });
        self.texture_cache.insert(index, asset.clone());
        Ok(asset)
    }

    fn load_texture_info(
        &mut self,
        info: texture::Info,
    ) -> Result<TextureInfo, GltfLoaderError<E>> {
        Ok(TextureInfo {
            texture: self.load_texture(info.texture())?,
            tex_coord: info.tex_coord() as usize,
            transform: info
                .texture_transform()
                .map(|transform| TextureAssetTransform {
                    offset: transform.offset(),
                    rotation: transform.rotation(),
                    scale: transform.scale(),
                    tex_coord: transform.tex_coord().map(|v| v as usize),
                }),
        })
    }

    fn load_texture_transform_from_value(
        transform: &Value,
    ) -> Result<TextureAssetTransform, GltfLoaderError<E>> {
        Ok(TextureAssetTransform {
            offset: transform
                .get("offset")
                .map(Self::load_vec2_value)
                .transpose()?
                .unwrap_or([0.0, 0.0]),
            rotation: transform
                .get("rotation")
                .map(|value| {
                    value
                        .as_f64()
                        .map(|v| v as f32)
                        .ok_or(GltfLoaderError::BadMToonData)
                })
                .transpose()?
                .unwrap_or(0.0),
            scale: transform
                .get("offset")
                .map(Self::load_vec2_value)
                .transpose()?
                .unwrap_or([1.0, 1.0]),
            tex_coord: transform
                .get("rotation")
                .map(|value| {
                    value
                        .as_u64()
                        .map(|v| v as usize)
                        .ok_or(GltfLoaderError::BadMToonData)
                })
                .transpose()?,
        })
    }

    fn load_texture_info_from_value(
        &mut self,
        value: &Value,
    ) -> Result<TextureInfo, GltfLoaderError<E>> {
        let index = value
            .get("index")
            .ok_or(GltfLoaderError::BadMToonData)?
            .as_u64()
            .ok_or(GltfLoaderError::BadMToonData)? as usize;
        let texture = self
            .document
            .textures()
            .nth(index)
            .ok_or(GltfLoaderError::BadMToonData)?;
        Ok(TextureInfo {
            texture: self.load_texture(texture)?,
            transform: value
                .get("transform")
                .map(Self::load_texture_transform_from_value)
                .transpose()?,
            tex_coord: value
                .get("texCoord")
                .map(|tex_coord| tex_coord.as_u64().ok_or(GltfLoaderError::BadMToonData))
                .transpose()?
                .map(|v| v as usize)
                .unwrap_or(0),
        })
    }

    fn load_shading_shift_texture_info_from_value(
        &mut self,
        value: &Value,
    ) -> Result<ShadingShiftTextureInfo, GltfLoaderError<E>> {
        let index = value
            .get("index")
            .ok_or(GltfLoaderError::BadMToonData)?
            .as_u64()
            .ok_or(GltfLoaderError::BadMToonData)? as usize;
        let texture = self
            .document
            .textures()
            .nth(index)
            .ok_or(GltfLoaderError::BadMToonData)?;
        Ok(ShadingShiftTextureInfo {
            texture: self.load_texture(texture)?,
            tex_coord: value
                .get("texCoord")
                .map(|tex_coord| tex_coord.as_u64().ok_or(GltfLoaderError::BadMToonData))
                .transpose()?
                .map(|v| v as usize)
                .unwrap_or(0),
            scale: value
                .get("scale")
                .map(|tex_coord| tex_coord.as_f64().ok_or(GltfLoaderError::BadMToonData))
                .transpose()?
                .map(|v| v as f32)
                .unwrap_or(1.0),
        })
    }

    fn load_uv_animation_from_value(
        &mut self,
        value: &Value,
    ) -> Result<UvAnimation, GltfLoaderError<E>> {
        let mask_texture_index = value
            .get("uvAnimationMaskTexture")
            .map(|index| index.as_u64().ok_or(GltfLoaderError::BadMToonData))
            .transpose()?
            .map(|v| v as usize);
        let mask_texture = mask_texture_index
            .map(|index| {
                self.document
                    .textures()
                    .nth(index)
                    .ok_or(GltfLoaderError::BadMToonData)
            })
            .transpose()?
            .map(|texture| self.load_texture(texture))
            .transpose()?;

        let scroll_x_speed_factor = value
            .get("uvAnimationScrollXSpeedFactor")
            .map(|factor| factor.as_f64().ok_or(GltfLoaderError::BadMToonData))
            .transpose()?
            .map(|v| v as f32)
            .unwrap_or(0.0);
        let scroll_y_speed_factor = value
            .get("uvAnimationScrollYSpeedFactor")
            .map(|factor| factor.as_f64().ok_or(GltfLoaderError::BadMToonData))
            .transpose()?
            .map(|v| v as f32)
            .unwrap_or(0.0);
        let rotation_speed_factor = value
            .get("uvAnimationRotationSpeedFactor")
            .map(|factor| factor.as_f64().ok_or(GltfLoaderError::BadMToonData))
            .transpose()?
            .map(|v| v as f32)
            .unwrap_or(0.0);

        Ok(UvAnimation {
            mask_texture,
            scroll_x_speed_factor,
            scroll_y_speed_factor,
            rotation_speed_factor,
        })
    }

    fn load_material(
        &mut self,
        material: Material,
    ) -> Result<Option<Arc<MaterialAsset>>, GltfLoaderError<E>> {
        let Some(index) = material.index() else {
            return Ok(None);
        };
        if let Some(material) = self.material_cache.get(&index) {
            return Ok(Some(material.clone()));
        }

        let alpha_mode = match material.alpha_mode() {
            AlphaMode::Opaque => MaterialAlphaMode::Opaque,
            AlphaMode::Mask => MaterialAlphaMode::Mask(material.alpha_cutoff().unwrap_or(0.5)),
            AlphaMode::Blend => MaterialAlphaMode::Blend,
        };

        let data = if let Some(mtoon) = material.extension_value("VRMC_materials_mtoon") {
            let pbr = material.pbr_metallic_roughness();
            MaterialAssetData::MTone {
                base_color_factor: pbr.base_color_factor(),
                base_color_texture: pbr
                    .base_color_texture()
                    .map(|value| self.load_texture_info(value))
                    .transpose()?,
                transparent_with_z_write: mtoon
                    .get("transparentWithZWrite")
                    .map(|value| value.as_bool().ok_or(GltfLoaderError::BadMToonData))
                    .transpose()?
                    .unwrap_or(false),
                render_queue_offset_number: mtoon
                    .get("renderQueueOffsetNumber")
                    .map(|value| value.as_i64().ok_or(GltfLoaderError::BadMToonData))
                    .transpose()?
                    .map(|v| v as isize)
                    .unwrap_or(0),
                shade_color_factor: mtoon
                    .get("shadeColorFactor")
                    .map(Self::load_vec3_value)
                    .transpose()?
                    .unwrap_or([0.0, 0.0, 0.0]),
                shade_multiply_texture: mtoon
                    .get("shadeMultiplyTexture")
                    .map(|value| self.load_texture_info_from_value(value))
                    .transpose()?,
                shading_shift_factor: mtoon
                    .get("shadingShiftFactor")
                    .map(|value| value.as_f64().ok_or(GltfLoaderError::BadMToonData))
                    .transpose()?
                    .map(|value| value as f32)
                    .unwrap_or(0.0),
                shading_shift_texture: mtoon
                    .get("shadingShiftTexture")
                    .map(|value| self.load_shading_shift_texture_info_from_value(value))
                    .transpose()?,
                shading_toony_factor: mtoon
                    .get("shadingToonyFactor")
                    .map(|value| value.as_f64().ok_or(GltfLoaderError::BadMToonData))
                    .transpose()?
                    .map(|value| value as f32)
                    .unwrap_or(0.9),
                gi_equalization_factor: mtoon
                    .get("giEqualizationFactor")
                    .map(|value| value.as_f64().ok_or(GltfLoaderError::BadMToonData))
                    .transpose()?
                    .map(|value| value as f32)
                    .unwrap_or(0.9),
                matcap_factor: mtoon
                    .get("matcapFactor")
                    .map(Self::load_vec3_value)
                    .transpose()?
                    .unwrap_or([1.0, 1.0, 1.0]),
                matcap_texture: mtoon
                    .get("matcapTexture")
                    .map(|texture| self.load_texture_info_from_value(texture))
                    .transpose()?,
                parametric_rim_color_factor: mtoon
                    .get("parametricRimColorFactor")
                    .map(Self::load_vec3_value)
                    .transpose()?
                    .unwrap_or([0.0, 0.0, 0.0]),
                parametric_rim_fresnel_power_factor: mtoon
                    .get("parametricRimFresnelPowerFactor")
                    .map(|value| value.as_f64().ok_or(GltfLoaderError::BadMToonData))
                    .transpose()?
                    .map(|value| value as f32)
                    .unwrap_or(5.0),
                parametric_rim_lift_factor: mtoon
                    .get("parametricRimLiftFactor")
                    .map(|value| value.as_f64().ok_or(GltfLoaderError::BadMToonData))
                    .transpose()?
                    .map(|value| value as f32)
                    .unwrap_or(0.0),
                rim_multiply_texture: mtoon
                    .get("rimMultiplyTexture")
                    .map(|texture| self.load_texture_info_from_value(texture))
                    .transpose()?,
                rim_lighting_mix_factor: mtoon
                    .get("rimLightingMixFactor")
                    .map(|value| value.as_f64().ok_or(GltfLoaderError::BadMToonData))
                    .transpose()?
                    .map(|value| value as f32)
                    .unwrap_or(1.0),
                outline_width_mode: mtoon
                    .get("outlineWidthMode")
                    .map(|value| match value.as_str() {
                        Some("none") => Ok(OutlineWidthMode::None),
                        Some("worldCoordinates") => Ok(OutlineWidthMode::WorldCoordinates),
                        Some("screenCoordinates") => Ok(OutlineWidthMode::ScreenCoordinates),
                        _ => Err(GltfLoaderError::BadMToonData),
                    })
                    .transpose()?
                    .unwrap_or(OutlineWidthMode::None),
                outline_width_factor: mtoon
                    .get("outlineWidthFactor")
                    .map(|value| value.as_f64().ok_or(GltfLoaderError::BadMToonData))
                    .transpose()?
                    .map(|value| value as f32)
                    .unwrap_or(0.0),
                outline_width_multiply_texture: mtoon
                    .get("outlineWidthMultiplyTexture")
                    .map(|texture| self.load_texture_info_from_value(texture))
                    .transpose()?,
                outline_color_factor: mtoon
                    .get("outlineColorFactor")
                    .map(Self::load_vec3_value)
                    .transpose()?
                    .unwrap_or([0.0, 0.0, 0.0]),
                outline_lighting_mix_factor: mtoon
                    .get("outlineLightingMixFactor")
                    .map(|value| {
                        value
                            .as_f64()
                            .map(|v| v as f32)
                            .ok_or(GltfLoaderError::BadMToonData)
                    })
                    .transpose()?
                    .unwrap_or(1.0),
            }
        } else if material.unlit() && !self.params.disable_unlit {
            let pbr = material.pbr_metallic_roughness();
            MaterialAssetData::Unlit {
                base_color_factor: pbr.base_color_factor(),
                base_color_texture: pbr
                    .base_color_texture()
                    .map(|texture| self.load_texture_info(texture))
                    .transpose()?,
            }
        } else {
            let pbr = material.pbr_metallic_roughness();
            MaterialAssetData::Pbr {
                base_color_factor: pbr.base_color_factor(),
                base_color_texture: pbr
                    .base_color_texture()
                    .map(|texture| self.load_texture_info(texture))
                    .transpose()?,
                metallic_factor: pbr.metallic_factor(),
                roughness_factor: pbr.roughness_factor(),
                metallic_roughness_texture: pbr
                    .metallic_roughness_texture()
                    .map(|texture| self.load_texture_info(texture))
                    .transpose()?,
            }
        };

        let normal_texture = material
            .normal_texture()
            .map(|info| {
                Ok::<_, GltfLoaderError<E>>(NormalTextureInfo {
                    texture: self.load_texture(info.texture())?,
                    tex_coord: info.tex_coord() as usize,
                    scale: info.scale(),
                })
            })
            .transpose()?;

        let occlusion_texture = material
            .occlusion_texture()
            .map(|info| {
                Ok::<_, GltfLoaderError<E>>(OcclusionTextureInfo {
                    texture: self.load_texture(info.texture())?,
                    tex_coord: info.tex_coord() as usize,
                    strength: info.strength(),
                })
            })
            .transpose()?;

        let emissive_texture = material
            .emissive_texture()
            .map(|info| self.load_texture_info(info))
            .transpose()?;

        let uv_animation = material
            .extension_value("VRMC_materials_mtoon")
            .map(|mtoon| self.load_uv_animation_from_value(mtoon))
            .transpose()?;

        let material = Arc::new(MaterialAsset {
            id: AssetIndex::BundleTypeIndex(
                self.data.bundle_index.clone(),
                BundleAssetType::Material,
                index,
            ),
            name: material.name().map(str::to_string),
            alpha_mode,
            data,
            normal_texture,
            occlusion_texture,
            emissive_texture,
            emissive_factor: material.emissive_factor(),
            double_sided: material.double_sided(),
            uv_animation,
        });
        self.material_cache.insert(index, material.clone());

        Ok(Some(material))
    }

    fn load_primitive_morph_target(
        &self,
        target: MorphTarget,
        mode: PrimitiveAssetMode,
        indices: Option<&[u32]>,
    ) -> Result<PrimitiveAssetMorphTarget, GltfLoaderError<E>> {
        let position = target
            .positions()
            .map(|accessor| {
                Self::check_accessor(&accessor, DataType::F32, Dimensions::Vec3)?;
                let data = Self::load_accessor_f32(self.data, &accessor);
                Ok::<_, GltfLoaderError<E>>(chunk_vec3(&data))
            })
            .transpose()?;
        let normal = target
            .normals()
            .map(|accessor| {
                Self::check_accessor(&accessor, DataType::F32, Dimensions::Vec3)?;
                let data = Self::load_accessor_f32(self.data, &accessor);
                Ok::<_, GltfLoaderError<E>>(chunk_vec3(&data))
            })
            .transpose()?
            .unwrap_or_default();
        let tangent = target
            .tangents()
            .map(|accessor| {
                Self::check_accessor(&accessor, DataType::F32, Dimensions::Vec4)?;
                let data = Self::load_accessor_f32(self.data, &accessor);
                Ok::<_, GltfLoaderError<E>>(chunk_vec4(&data))
            })
            .transpose()?
            .or_else(|| {
                position.as_ref().map(|position| {
                    pad_vec3_to_vec4(&calculate_tangent(mode, position.as_slice(), indices), 1.0)
                })
            })
            .unwrap_or_default();

        Ok(PrimitiveAssetMorphTarget {
            position: position.unwrap_or_default(),
            normal,
            tangent,
        })
    }

    fn load_primitive(
        &mut self,
        primitive: Primitive,
    ) -> Result<PrimitiveAsset, GltfLoaderError<E>> {
        let material = self.load_material(primitive.material())?;

        let indices = primitive
            .indices()
            .map(|accessor| match accessor.data_type() {
                DataType::U8 => Ok(Self::load_accessor_u8(self.data, &accessor)
                    .into_iter()
                    .map(|item| item as u32)
                    .collect()),
                DataType::U16 => Ok(Self::load_accessor_u16(self.data, &accessor)
                    .into_iter()
                    .map(|item| item as u32)
                    .collect()),
                DataType::U32 => Ok(Self::load_accessor_u32(self.data, &accessor)),
                _ => Err(GltfLoaderError::BadAccessorDataType(
                    DataType::U16,
                    accessor.data_type(),
                )),
            })
            .transpose()?;

        let mut position: Option<Position> = None;
        let mut normal: Option<Normal> = None;
        let mut tangent: Option<Tangent> = None;
        let mut tex_coords = Vec::new();
        let mut color = Vec::new();
        let mut joints = Vec::new();
        let mut weights = Vec::new();

        fn ensure_size<T>(vec: &mut Vec<Option<T>>, size: usize) {
            if size > vec.len() {
                vec.resize_with(size, || None);
            }
        }

        for (semantic, accessor) in primitive.attributes() {
            match semantic {
                Semantic::Positions => {
                    Self::check_accessor(&accessor, DataType::F32, Dimensions::Vec3)?;
                    let data = Self::load_accessor_f32(self.data, &accessor);
                    position = Some(chunk_vec3(&data));
                }
                Semantic::Normals => {
                    Self::check_accessor(&accessor, DataType::F32, Dimensions::Vec3)?;
                    let data = Self::load_accessor_f32(self.data, &accessor);
                    normal = Some(chunk_vec3(&data));
                }
                Semantic::Tangents => {
                    Self::check_accessor(&accessor, DataType::F32, Dimensions::Vec4)?;
                    let data = Self::load_accessor_f32(self.data, &accessor);
                    tangent = Some(chunk_vec4(&data));
                }
                Semantic::Colors(index) => {
                    ensure_size(&mut color, index as usize + 1);
                    let data = Self::load_accessor_normalized(self.data, &accessor);
                    let color_item = match accessor.dimensions() {
                        Dimensions::Vec3 => chunk_and_clamp_vec3_to_vec4_f32(&data),
                        Dimensions::Vec4 => chunk_and_clamp_vec4_f32(&data),
                        _ => {
                            return Err(GltfLoaderError::BadAccessorDimensions(
                                Dimensions::Vec4,
                                accessor.dimensions(),
                            ))
                        }
                    };
                    color[index as usize] = Some(color_item);
                }
                Semantic::TexCoords(index) => {
                    ensure_size(&mut tex_coords, index as usize + 1);
                    if accessor.dimensions() != Dimensions::Vec2 {
                        return Err(GltfLoaderError::BadAccessorDimensions(
                            Dimensions::Vec2,
                            accessor.dimensions(),
                        ));
                    }
                    let data = Self::load_accessor_normalized(self.data, &accessor);
                    let coords: Vec<_> = data
                        .chunks_exact(2)
                        .map(|chunk| chunk.try_into().unwrap())
                        .collect();
                    tex_coords[index as usize] = Some(coords);
                }
                Semantic::Joints(index) => {
                    ensure_size(&mut joints, index as usize + 1);
                    if accessor.dimensions() != Dimensions::Vec4 {
                        return Err(GltfLoaderError::BadAccessorDimensions(
                            Dimensions::Vec4,
                            accessor.dimensions(),
                        ));
                    }
                    let data = match accessor.data_type() {
                        DataType::U8 => Self::load_accessor_u8(self.data, &accessor)
                            .into_iter()
                            .map(|num| num as u16)
                            .collect(),
                        DataType::U16 => Self::load_accessor_u16(self.data, &accessor),
                        _ => {
                            return Err(GltfLoaderError::BadAccessorDataType(
                                DataType::U16,
                                accessor.data_type(),
                            ))
                        }
                    };
                    let data = chunk_vec4(&data);
                    joints[index as usize] = Some(data);
                }
                Semantic::Weights(index) => {
                    ensure_size(&mut weights, index as usize + 1);
                    if accessor.dimensions() != Dimensions::Vec4 {
                        return Err(GltfLoaderError::BadAccessorDimensions(
                            Dimensions::Vec4,
                            accessor.dimensions(),
                        ));
                    }
                    let data = Self::load_accessor_normalized(self.data, &accessor);
                    let data = chunk_vec4(&data);
                    weights[index as usize] = Some(data);
                }
            }
        }

        let position = position.expect("No positions in primitive");
        let tex_coord: Vec<TexCoord> = tex_coords
            .into_iter()
            .map(|tex_coords| tex_coords.expect("Missing texture coordinates set"))
            .collect();
        let color: Vec<VertexColor> = color
            .into_iter()
            .map(|vertex_colors| vertex_colors.expect("Missing vertex color set"))
            .collect();
        let joints: Vec<Joints> = joints
            .into_iter()
            .map(|vertex_colors| vertex_colors.expect("Missing skin joints set"))
            .collect();
        let weights: Vec<Weights> = weights
            .into_iter()
            .map(|vertex_colors| vertex_colors.expect("Missing skin weights set"))
            .collect();
        if joints.len() != weights.len() {
            return Err(GltfLoaderError::BadJointWeightSets(
                joints.len(),
                weights.len(),
            ));
        }

        let mode = match primitive.mode() {
            Mode::Points => PrimitiveAssetMode::Points,
            Mode::Lines => PrimitiveAssetMode::LineList,
            Mode::LineStrip => PrimitiveAssetMode::LineStrip,
            Mode::Triangles => PrimitiveAssetMode::TriangleList,
            Mode::TriangleStrip => PrimitiveAssetMode::TriangleStrip,
            unsupported => return Err(GltfLoaderError::UnsupportedPrimitiveMode(unsupported)),
        };
        let tangent = tangent.unwrap_or_else(|| {
            pad_vec3_to_vec4(&calculate_tangent(mode, &position, indices.as_deref()), 1.0)
        });
        let normal = normal.unwrap_or_else(|| clip_vec4_to_vec3(&tangent));

        let targets = primitive
            .morph_targets()
            .map(|target| self.load_primitive_morph_target(target, mode, indices.as_deref()))
            .collect::<Result<_, _>>()?;

        Ok(PrimitiveAsset {
            attributes: PrimitiveAssetAttributes {
                position,
                normal,
                tangent,
                tex_coord,
                color,
                joints,
                weights,
            },
            indices,
            material,
            mode,
            targets,
        })
    }

    fn load_mesh(&mut self, mesh: Mesh) -> Result<MeshAsset, GltfLoaderError<E>> {
        let primitives = mesh
            .primitives()
            .map(|primitive| self.load_primitive(primitive))
            .collect::<Result<_, _>>()?;
        Ok(MeshAsset {
            name: mesh.name().map(str::to_string),
            primitives,
            weights: mesh
                .weights()
                .map(|weights| weights.to_vec())
                .unwrap_or_default(),
        })
    }

    fn load_skin(&mut self, skin: &Skin) -> Arc<SkinAsset> {
        if let Some(skin) = self.skin_cache.get(&skin.index()) {
            return skin.clone();
        }

        let joint_ids: Vec<AssetIndex> = skin
            .joints()
            .map(|joint| {
                AssetIndex::BundleTypeIndex(
                    self.data.bundle_index.clone(),
                    BundleAssetType::Node,
                    joint.index(),
                )
            })
            .collect();
        let inverse_bind_matrices = skin
            .inverse_bind_matrices()
            .map(|accessor| {
                let data = Self::load_accessor_f32(self.data, &accessor);
                chunk_mat4(&data)
            })
            .unwrap_or_default();
        let skeleton = skin.skeleton().map(|skeleton| {
            AssetIndex::BundleTypeIndex(
                self.data.bundle_index.clone(),
                BundleAssetType::Node,
                skeleton.index(),
            )
        });

        let skin_asset = Arc::new(SkinAsset {
            id: AssetIndex::BundleTypeIndex(
                self.data.bundle_index.clone(),
                BundleAssetType::Skin,
                skin.index(),
            ),
            joint_ids,
            inverse_bind_matrices,
            skeleton,
        });
        self.skin_cache.insert(skin.index(), skin_asset.clone());
        skin_asset
    }

    fn load_node(&mut self, node: Node) -> Result<NodeAsset, GltfLoaderError<E>> {
        let id = AssetIndex::BundleTypeIndex(
            self.data.bundle_index.clone(),
            BundleAssetType::Node,
            node.index(),
        );
        let transform = match node.transform() {
            Transform::Matrix { matrix } => {
                NodeTransform::Matrix(MatrixNodeTransform(Mat4::from_cols_array_2d(&matrix)))
            }
            Transform::Decomposed {
                translation,
                rotation,
                scale,
            } => NodeTransform::Decomposed(DecomposedTransform {
                translation: Vec3::from_array(translation),
                rotation: Quat::from_array(rotation),
                scale: Vec3::from_array(scale),
            }),
        };
        let mesh = node.mesh().map(|mesh| self.load_mesh(mesh)).transpose()?;
        let skin = node.skin().map(|skin| self.load_skin(&skin));
        let camera = node.camera().map(Self::load_camera);
        let children = node
            .children()
            .map(|child| self.load_node(child))
            .collect::<Result<_, _>>()?;
        let weights = node.weights().unwrap_or_default().to_vec();

        Ok(NodeAsset {
            id,
            name: node.name().map(str::to_string),
            transform: Some(transform),
            mesh,
            skin,
            camera,
            children,
            weights,
        })
    }

    fn load_scene(&mut self, scene: Scene) -> Result<SceneAsset, GltfLoaderError<E>> {
        Ok(SceneAsset {
            name: scene.name().map(str::to_string),
            nodes: scene
                .nodes()
                .map(|node| self.load_node(node))
                .collect::<Result<_, _>>()?,
        })
    }

    fn load_animation_sampler(
        &self,
        sampler: Sampler,
        path_type: AnimationPathType,
    ) -> (AnimationSampler, f32) {
        let time = Self::load_accessor_f32(self.data, &sampler.input());

        fn read_keyframes<T: Debug + Clone>(
            time: Vec<f32>,
            data: Vec<T>,
            repeat_time: bool,
        ) -> (Vec<AnimationKeyFrame<T>>, f32) {
            let keyframes: Vec<AnimationKeyFrame<T>> = if repeat_time {
                time.into_iter()
                    .flat_map(|time| iter::repeat(time).take(3))
                    .zip(data)
                    .map(|(time, value)| AnimationKeyFrame { time, value })
                    .collect()
            } else {
                time.into_iter()
                    .zip(data)
                    .map(|(time, value)| AnimationKeyFrame { time, value })
                    .collect()
            };
            let length = keyframes
                .iter()
                .max_by(|x, y| x.time.total_cmp(&y.time))
                .map(|item| item.time)
                .unwrap_or(0.0);
            (keyframes, length)
        }

        fn split_keyframes<T: Debug + Clone>(
            keyframes: Vec<AnimationKeyFrame<T>>,
        ) -> Vec<AnimationKeyFrame<(T, T, T)>> {
            let split = keyframes
                .chunks_exact(3)
                .map(|chunks| AnimationKeyFrame {
                    time: chunks[0].time,
                    value: (
                        chunks[0].value.clone(),
                        chunks[1].value.clone(),
                        chunks[2].value.clone(),
                    ),
                })
                .collect();
            split
        }

        fn interpolate_frames<T: Debug + Clone>(
            keyframes: Vec<AnimationKeyFrame<T>>,
            interpolation: Interpolation,
        ) -> AnimationKeyFrames<T> {
            match interpolation {
                Interpolation::Linear => AnimationKeyFrames::Linear(keyframes),
                Interpolation::Step => AnimationKeyFrames::Step(keyframes),
                Interpolation::CubicSpline => {
                    AnimationKeyFrames::CubicSpline(split_keyframes(keyframes))
                }
            }
        }

        match path_type {
            AnimationPathType::Rotation => {
                let data = Self::load_accessor_normalized(self.data, &sampler.output());
                let repeat_times = sampler.interpolation() == Interpolation::CubicSpline;
                let (keyframes, length) = read_keyframes(time, chunk_vec4(&data), repeat_times);
                let keyframes = interpolate_frames(keyframes, sampler.interpolation());
                (AnimationSampler::Rotation(keyframes), length)
            }
            AnimationPathType::Translation => {
                let data = Self::load_accessor_f32(self.data, &sampler.output());
                let repeat_times = sampler.interpolation() == Interpolation::CubicSpline;
                let (keyframes, length) = read_keyframes(time, chunk_vec3(&data), repeat_times);
                let keyframes = interpolate_frames(keyframes, sampler.interpolation());
                (AnimationSampler::Translation(keyframes), length)
            }
            AnimationPathType::Scale => {
                let data = Self::load_accessor_f32(self.data, &sampler.output());
                let repeat_times = sampler.interpolation() == Interpolation::CubicSpline;
                let (keyframes, length) = read_keyframes(time, chunk_vec3(&data), repeat_times);
                let keyframes = interpolate_frames(keyframes, sampler.interpolation());
                (AnimationSampler::Scale(keyframes), length)
            }
        }
    }

    fn load_animation_channel(&mut self, channel: Channel) -> AnimationChannelAsset {
        let target = channel.target();
        let path_type = match target.property() {
            Property::Translation => AnimationPathType::Translation,
            Property::Rotation => AnimationPathType::Rotation,
            Property::Scale => AnimationPathType::Scale,
            Property::MorphTargetWeights => todo!(),
        };
        let (sampler, length) = self.load_animation_sampler(channel.sampler(), path_type);
        let target = target.node();
        let target_id = AssetIndex::BundleTypeIndex(
            self.data.bundle_index.clone(),
            BundleAssetType::Node,
            target.index(),
        );
        AnimationChannelAsset {
            sampler,
            length,
            target_id,
        }
    }

    fn load_animation(&mut self, animation: Animation) -> AnimationAsset {
        let channels = animation
            .channels()
            .map(|channel| self.load_animation_channel(channel))
            .collect();
        AnimationAsset {
            name: animation.name().map(str::to_string),
            channels,
        }
    }

    fn load(&mut self) -> GLTFLoadResult<E> {
        let animations = self
            .document
            .animations()
            .map(|scene| self.load_animation(scene))
            .collect();
        let scenes = self
            .document
            .scenes()
            .map(|scene| self.load_scene(scene))
            .collect::<Result<_, _>>()?;
        Ok((scenes, animations))
    }
}

/// Load a GLB file from a slice.
///
/// BundleIndex will be generated by hashing the content of the slice.
#[cfg(feature = "digest")]
pub fn load_glb_from_buffer(buffer: &[u8], params: &AssetLoadParams) -> GLTFLoadResult<io::Error> {
    let id = BundleIndex::digest_from_buffer(buffer);
    load_glb_from_buffer_with_id(buffer, id, params)
}

/// Load a GLB file from a slice, with a BundleIndex specified.
pub fn load_glb_from_buffer_with_id(
    buffer: &[u8],
    id: BundleIndex,
    params: &AssetLoadParams,
) -> GLTFLoadResult<io::Error> {
    let (document, buffers, images) = gltf::import_slice(buffer)?;
    let data = GltfData {
        bundle_index: id,
        buffers,
        images,
    };
    let mut loader = GltfDocumentLoader::new(&document, &data, params);
    loader.load()
}

/// Load a GLTF file from archive.
///
/// The archive will be used to read binary buffers and images. Only data URI
/// and absolute (file://) and relative paths are supported. Other URIs such as
/// HTTP are not accepted and will cause [`GltfLoaderError::InvalidScheme`].
pub fn load_gltf_from_archive<T, A: Archive<T>>(
    archive: &mut A,
    id: BundleIndex,
    params: &AssetLoadParams,
) -> GLTFLoadResult<A::Error> {
    let file_name = params.bundle_model_filename("gltf");

    let mut gltf_entry = archive
        .by_path(&file_name)
        .map_err(GltfLoaderError::Io)?
        .ok_or_else(|| GltfLoaderError::ModelNotFound(file_name.clone()))?;
    let gltf_data = gltf_entry.unpack().map_err(GltfLoaderError::Io)?;
    drop(gltf_entry);
    let gltf = Gltf::from_slice(&gltf_data)?;

    let mut buffers = Vec::new();
    for buffer in gltf.buffers() {
        let uri = match buffer.source() {
            gltf::buffer::Source::Bin => return Err(GltfLoaderError::BadModelFile),
            gltf::buffer::Source::Uri(uri) => uri,
        };
        let scheme = Scheme::try_from(uri)?;

        // If MIME is specified, check the MIME
        if let Scheme::Data(mime, _) = &scheme {
            if let Some(mime) = mime {
                if !mime.eq_ignore_ascii_case("application/octet-stream")
                    && mime.eq_ignore_ascii_case("application/gltf-buffer")
                {
                    return Err(GltfLoaderError::BadBufferMime(
                        uri.to_string(),
                        Some(mime.to_string()),
                    ));
                }
            } else {
                return Err(GltfLoaderError::BadBufferMime(uri.to_string(), None));
            }
        }

        // Load data from archive
        let Some((_mime, mut data)) = scheme
            .load(archive, &file_name)
            .map_err(GltfLoaderError::Io)?
        else {
            return Err(GltfLoaderError::ResourceNotFound(uri.to_string()));
        };

        // Pad the data to 4 bytes with zeroes
        while data.len() % 4 != 0 {
            data.push(0);
        }

        buffers.push(gltf::buffer::Data(data));
    }

    let mut images = Vec::new();
    for (index, buffer) in gltf.images().enumerate() {
        fn load_image<E>(
            source: GltfImageSource,
            data: &[u8],
            format: ImageFormat,
        ) -> Result<gltf::image::Data, GltfLoaderError<E>> {
            let mut reader = ImageReader::new(Cursor::new(data));
            reader.set_format(format);
            let image = reader
                .decode()
                .map_err(|error| GltfLoaderError::BadImage(source.clone(), error))?;

            let format = match image {
                DynamicImage::ImageLuma8(_) => Format::R8,
                DynamicImage::ImageLumaA8(_) => Format::R8G8,
                DynamicImage::ImageRgb8(_) => Format::R8G8B8,
                DynamicImage::ImageRgba8(_) => Format::R8G8B8A8,
                DynamicImage::ImageLuma16(_) => Format::R16,
                DynamicImage::ImageLumaA16(_) => Format::R16G16,
                DynamicImage::ImageRgb16(_) => Format::R16G16B16,
                DynamicImage::ImageRgba16(_) => Format::R16G16B16A16,
                DynamicImage::ImageRgb32F(_) => Format::R32G32B32FLOAT,
                DynamicImage::ImageRgba32F(_) => Format::R32G32B32A32FLOAT,
                _unsupported => return Err(GltfLoaderError::BadImageFormat(source)),
            };
            let (width, height) = image.dimensions();
            let pixels = image.into_bytes();
            Ok(gltf::image::Data {
                format,
                width,
                height,
                pixels,
            })
        }

        match buffer.source() {
            gltf::image::Source::View {
                view,
                mime_type: mime,
            } => {
                let buffer_index = view.buffer().index();

                let data = buffers
                    .get(buffer_index)
                    .ok_or(GltfLoaderError::ImageBufferOutOfBounds(
                        index,
                        buffer_index,
                        buffers.len(),
                    ))?
                    .0
                    .as_slice();

                let image_format = ImageFormat::from_mime_type(mime).ok_or_else(|| {
                    GltfLoaderError::BadImageMime(
                        GltfImageSource::Buffer(buffer_index),
                        mime.to_string(),
                    )
                })?;

                let data = load_image(GltfImageSource::Buffer(buffer_index), data, image_format)?;
                images.push(data);
            }
            gltf::image::Source::Uri {
                uri,
                mime_type: mime,
            } => {
                let scheme = Scheme::try_from(uri)?;
                let Some((load_mime, data)) = scheme
                    .load(archive, &file_name)
                    .map_err(GltfLoaderError::Io)?
                else {
                    return Err(GltfLoaderError::ResourceNotFound(uri.to_string()));
                };

                let mime = mime.or(load_mime);
                let image_format = if let Some(mime) = mime {
                    ImageFormat::from_mime_type(mime).ok_or_else(|| {
                        GltfLoaderError::BadImageMime(
                            GltfImageSource::Uri(uri.to_string()),
                            mime.to_string(),
                        )
                    })?
                } else {
                    guess_format(data.as_slice()).map_err(|error| {
                        GltfLoaderError::BadImage(GltfImageSource::Uri(uri.to_string()), error)
                    })?
                };

                let data = load_image(
                    GltfImageSource::Uri(uri.to_string()),
                    data.as_slice(),
                    image_format,
                )?;
                images.push(data);
            }
        }
    }

    let data = GltfData {
        bundle_index: id,
        buffers,
        images,
    };
    let document = gltf.document;
    let mut loader = GltfDocumentLoader::new(&document, &data, params);
    loader.load()
}
