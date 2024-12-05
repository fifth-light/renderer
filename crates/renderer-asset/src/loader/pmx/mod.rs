use std::{
    fmt::{Display, Formatter},
    io::Cursor,
    marker::PhantomData,
    sync::Arc,
};

use binrw::BinRead;
use format::{PmxFile, PmxMaterial, PmxTexture};

use crate::{
    archive::{Archive, Entry},
    index::{AssetIndex, BundleAssetType, BundleIndex},
    material::{self, MaterialAlphaMode, MaterialAsset, MaterialAssetData},
    mesh::MeshAsset,
    node::NodeAsset,
    primitive::{PrimitiveAsset, PrimitiveAssetAttributes, PrimitiveAssetMode},
    scene::SceneAsset,
    tangent::calculate_tangent,
    texture::{SamplerAsset, TextureAsset, TextureInfo},
};

use super::{
    pad_vec3_to_vec4,
    texture::{TextureLoadError, TextureLoader},
    AssetLoadParams,
};

mod format;

#[derive(Debug)]
pub enum PmxLoadError<E> {
    Format(binrw::Error),
    Io(E),
    ModelNotFound(String),
    Texture(TextureLoadError<E>),
    NoSurfaceLeft { expected: usize, actual: usize },
    BadSurfacesCount(usize),
    BadToonReference(String),
}

impl<E: Display> Display for PmxLoadError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PmxLoadError::Format(format) => Display::fmt(format, f),
            PmxLoadError::Io(io) => Display::fmt(io, f),
            PmxLoadError::ModelNotFound(file_name) => {
                write!(f, "File {} not found in bundle", file_name)
            }
            PmxLoadError::Texture(texture) => Display::fmt(texture, f),
            PmxLoadError::NoSurfaceLeft { expected, actual } => write!(
                f,
                "Want to read {} surfaces for material, but only {} left",
                expected, actual
            ),
            PmxLoadError::BadSurfacesCount(count) => {
                write!(f, "Bad surfaces count in material: {}", count)
            }
            PmxLoadError::BadToonReference(material) => {
                write!(f, "Bad toon reference for material {:?}", material)
            }
        }
    }
}

impl<E: std::error::Error> std::error::Error for PmxLoadError<E> {}

impl<E> From<binrw::Error> for PmxLoadError<E> {
    fn from(value: binrw::Error) -> Self {
        Self::Format(value)
    }
}

struct PmxLoader<'a, T, A: Archive<T>> {
    id: BundleIndex,
    bundle: &'a mut A,
    texture_loader: TextureLoader,
    _markor: PhantomData<T>,
}

impl<'a, T, A: Archive<T>> PmxLoader<'a, T, A> {
    fn new(id: BundleIndex, bundle: &'a mut A) -> Self {
        Self {
            id,
            bundle,
            texture_loader: TextureLoader::default(),
            _markor: PhantomData,
        }
    }

    fn load_texture(
        &mut self,
        texture: &PmxTexture,
    ) -> Result<Option<Arc<TextureAsset>>, PmxLoadError<A::Error>> {
        let id = AssetIndex::BundlePath(self.id.clone(), texture.path.clone());
        let texture = self
            .texture_loader
            .load_from_archive(id, self.bundle, &texture.path, SamplerAsset::default())
            .map_err(PmxLoadError::Texture)?;
        Ok(texture)
    }

    fn load_texture_info(
        &mut self,
        texture: &PmxTexture,
    ) -> Result<Option<TextureInfo>, PmxLoadError<A::Error>> {
        let texture = self.load_texture(texture)?;
        Ok(texture.map(TextureInfo::from_texture))
    }

    fn load_material(
        &mut self,
        file: &PmxFile,
        index: usize,
        material: &PmxMaterial,
    ) -> Result<Arc<MaterialAsset>, PmxLoadError<A::Error>> {
        let texture = material
            .texture_index
            .0
            .map(|index| self.load_texture_info(&file.textures[index]))
            .transpose()?
            .flatten();
        let environment = material
            .environment_index
            .0
            .map(|index| self.load_texture_info(&file.textures[index]))
            .transpose()?
            .flatten();

        let data = MaterialAssetData::Pmx {
            no_cull: material.drawing_flags.no_cull(),
            ground_shadow: material.drawing_flags.ground_shadow(),
            draw_shadow: material.drawing_flags.draw_shadow(),
            receive_shadow: material.drawing_flags.receive_shadow(),
            has_edge: material.drawing_flags.has_edge(),
            ambient_color: material.ambient_color,
            diffuse_color: material.diffuse_color,
            specular_color: material.specular_color,
            specular_strength: material.specular_strength,
            edge_color: material.edge_color,
            edge_scale: material.edge_scale,
            texture,
            environment,
            environment_blend_mode: match material.environment_blend_mode {
                format::PmxEnvironmentBlendMode::Disabled => {
                    material::PmxEnvironmentBlendMode::Disabled
                }
                format::PmxEnvironmentBlendMode::Multiply => {
                    material::PmxEnvironmentBlendMode::Multiply
                }
                format::PmxEnvironmentBlendMode::Additive => {
                    material::PmxEnvironmentBlendMode::Additive
                }
                format::PmxEnvironmentBlendMode::AdditionalVec4 => {
                    material::PmxEnvironmentBlendMode::AdditionalVec4
                }
            },
            toon_reference: match material.toon_reference {
                format::PmxToonReference::Texture { index } => {
                    let texture = index
                        .0
                        .map(|index| self.load_texture(&file.textures[index]))
                        .transpose()?
                        .flatten();
                    if let Some(texture) = texture {
                        material::PmxToonReference::Texture(texture)
                    } else {
                        return Err(PmxLoadError::BadToonReference(
                            material.material_name_local.clone(),
                        ));
                    }
                }
                format::PmxToonReference::Internal { index } => {
                    material::PmxToonReference::Internal { index }
                }
            },
        };

        Ok(Arc::new(MaterialAsset {
            name: Some(material.material_name_local.clone()),
            id: AssetIndex::BundleTypeIndex(self.id.clone(), BundleAssetType::Material, index),
            data,
            normal_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            emissive_factor: [0.0, 0.0, 0.0],
            alpha_mode: MaterialAlphaMode::Opaque,
            double_sided: false,
            uv_animation: None,
        }))
    }

    fn surface_id(&self, index: usize) -> AssetIndex {
        AssetIndex::BundleTypeIndex(self.id.clone(), BundleAssetType::Node, index)
    }

    fn load_surfaces(&mut self, file: &PmxFile) -> Result<Vec<NodeAsset>, PmxLoadError<A::Error>> {
        let mut surfaces_next = file.surfaces.as_slice();
        let mut nodes = Vec::new();
        for (index, material) in file.materials.iter().enumerate() {
            let material_asset = self.load_material(file, index, material)?;
            let surface_count = material.surface_count as usize;
            if surfaces_next.len() < surface_count {
                return Err(PmxLoadError::NoSurfaceLeft {
                    expected: surface_count,
                    actual: surfaces_next.len(),
                });
            } else if surface_count % 3 != 0 {
                return Err(PmxLoadError::BadSurfacesCount(surface_count));
            }
            let (surfaces, surfaces_left) = surfaces_next.split_at(material.surface_count as usize);
            surfaces_next = surfaces_left;

            let surfaces = surfaces
                .iter()
                .map(|surface| &file.vertices[surface.0.unwrap()]);

            let mut position = Vec::new();
            let mut tex_coord = Vec::new();
            let mut normal = Vec::new();
            for surface in surfaces {
                position.push(surface.position);
                tex_coord.push(surface.uv);
                normal.push(surface.normal);
            }
            let tangent = pad_vec3_to_vec4(
                &calculate_tangent(PrimitiveAssetMode::TriangleList, &position, None),
                1.0,
            );
            let primitive = PrimitiveAsset {
                attributes: PrimitiveAssetAttributes {
                    position,
                    normal,
                    tangent,
                    tex_coord: vec![tex_coord],
                    color: vec![],
                    joints: vec![],
                    weights: vec![],
                },
                indices: None,
                material: Some(material_asset),
                mode: PrimitiveAssetMode::TriangleList,
                targets: vec![],
            };
            let mesh = MeshAsset {
                name: None,
                primitives: vec![primitive],
                weights: vec![],
            };
            let node = NodeAsset {
                id: self.surface_id(index),
                name: None,
                transform: None,
                mesh: Some(mesh),
                skin: None,
                camera: None,
                children: vec![],
                weights: vec![],
            };
            nodes.push(node);
        }

        Ok(nodes)
    }

    fn load_file(&mut self, file: PmxFile) -> Result<SceneAsset, PmxLoadError<A::Error>> {
        let surfaces = self.load_surfaces(&file)?;
        Ok(SceneAsset {
            name: None,
            nodes: surfaces,
        })
    }
}

pub fn load_bundle<T, A: Archive<T>>(
    id: BundleIndex,
    bundle: &mut A,
    params: AssetLoadParams,
) -> Result<SceneAsset, PmxLoadError<A::Error>> {
    let file_name = params.bundle_model_filename("pmx");

    let mut file_entry = bundle
        .by_path(&file_name)
        .map_err(PmxLoadError::Io)?
        .ok_or_else(|| PmxLoadError::ModelNotFound(file_name))?;
    let file = file_entry.unpack().map_err(PmxLoadError::Io)?;
    drop(file_entry);
    let file = PmxFile::read_le(&mut Cursor::new(file))?;

    let mut loader = PmxLoader::new(id, bundle);
    loader.load_file(file)
}
