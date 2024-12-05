use std::{
    cell::RefCell,
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    io::Cursor,
    marker::PhantomData,
    sync::Arc,
};

use tobj::{LoadError, LoadOptions, Material, Model};

use crate::{
    archive::{Archive, Entry},
    index::{AssetIndex, BundleAssetType, BundleIndex},
    material::{MaterialAlphaMode, MaterialAsset, MaterialAssetData},
    mesh::MeshAsset,
    primitive::{PrimitiveAsset, PrimitiveAssetAttributes, PrimitiveAssetMode},
    tangent::calculate_tangent,
    texture::{NormalTextureInfo, SamplerAsset, TextureInfo},
};

use super::{
    chunk_vec3, pad_vec3_to_vec4,
    texture::{TextureLoadError, TextureLoader},
    AssetLoadParams,
};

#[derive(Debug)]
pub enum ObjLoadError<E> {
    Obj(LoadError),
    Io(E),
    ModelNotFound(String),
    Texture(TextureLoadError<E>),
}

impl<E: Display> Display for ObjLoadError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ObjLoadError::Obj(err) => Display::fmt(&err, f),
            ObjLoadError::Texture(err) => Display::fmt(&err, f),
            ObjLoadError::ModelNotFound(file_name) => {
                write!(f, "File {} not found in bundle", file_name)
            }
            ObjLoadError::Io(err) => Display::fmt(err, f),
        }
    }
}

impl<E: Error> Error for ObjLoadError<E> {}

impl<E> From<LoadError> for ObjLoadError<E> {
    fn from(value: LoadError) -> Self {
        ObjLoadError::Obj(value)
    }
}

impl<E> From<TextureLoadError<E>> for ObjLoadError<E> {
    fn from(value: TextureLoadError<E>) -> Self {
        ObjLoadError::Texture(value)
    }
}

struct ObjLoader<'a, T, A: Archive<T>> {
    id: BundleIndex,
    bundle: &'a mut A,
    texture_loader: TextureLoader,
    materials: &'a [Material],
    material_cache: HashMap<usize, Arc<MaterialAsset>>,
    _marker: PhantomData<T>,
}

impl<'a, T, A: Archive<T>> ObjLoader<'a, T, A> {
    pub fn new(id: BundleIndex, bundle: &'a mut A, materials: &'a [Material]) -> Self {
        Self {
            id,
            bundle,
            texture_loader: TextureLoader::default(),
            materials,
            material_cache: HashMap::new(),
            _marker: PhantomData,
        }
    }

    pub fn load_material(
        &mut self,
        index: usize,
    ) -> Result<Option<Arc<MaterialAsset>>, ObjLoadError<A::Error>> {
        if let Some(material) = self.material_cache.get(&index) {
            return Ok(Some(material.clone()));
        }
        let Some(material) = self.materials.get(index) else {
            return Ok(None);
        };

        let ambient_color = material.ambient.unwrap_or([1.0, 1.0, 1.0]);
        let diffuse_color = material.diffuse.unwrap_or([1.0, 1.0, 1.0]);
        let specular_color = material.specular.unwrap_or([1.0, 1.0, 1.0]);
        let shininess = material.shininess.unwrap_or(1.0);
        let dissolve = material.dissolve.unwrap_or(1.0);
        let optical_density = material.optical_density.unwrap_or(1.0);
        let ambient_texture = material
            .ambient_texture
            .as_ref()
            .map(|ambient_texture| {
                self.texture_loader.load_from_archive(
                    AssetIndex::BundlePath(self.id.clone(), ambient_texture.clone()),
                    self.bundle,
                    ambient_texture,
                    SamplerAsset::default(),
                )
            })
            .transpose()?
            .flatten();
        let diffuse_texture = material
            .diffuse_texture
            .as_ref()
            .map(|diffuse_texture| {
                self.texture_loader.load_from_archive(
                    AssetIndex::BundlePath(self.id.clone(), diffuse_texture.clone()),
                    self.bundle,
                    diffuse_texture,
                    SamplerAsset::default(),
                )
            })
            .transpose()?
            .flatten();
        let specular_texture = material
            .specular_texture
            .as_ref()
            .map(|specular_texture| {
                self.texture_loader.load_from_archive(
                    AssetIndex::BundlePath(self.id.clone(), specular_texture.clone()),
                    self.bundle,
                    specular_texture,
                    SamplerAsset::default(),
                )
            })
            .transpose()?
            .flatten();
        let shininess_texture = material
            .shininess_texture
            .as_ref()
            .map(|shininess_texture| {
                self.texture_loader.load_from_archive(
                    AssetIndex::BundlePath(self.id.clone(), shininess_texture.clone()),
                    self.bundle,
                    shininess_texture,
                    SamplerAsset::default(),
                )
            })
            .transpose()?
            .flatten();
        let dissolve_texture = material
            .dissolve_texture
            .as_ref()
            .map(|dissolve_texture| {
                self.texture_loader.load_from_archive(
                    AssetIndex::BundlePath(self.id.clone(), dissolve_texture.clone()),
                    self.bundle,
                    dissolve_texture,
                    SamplerAsset::default(),
                )
            })
            .transpose()?
            .flatten();
        let normal_texture = material
            .normal_texture
            .as_ref()
            .map(|normal_texture| {
                self.texture_loader.load_from_archive(
                    AssetIndex::BundlePath(self.id.clone(), normal_texture.clone()),
                    self.bundle,
                    normal_texture,
                    SamplerAsset::default(),
                )
            })
            .transpose()?
            .flatten();

        let material = Arc::new(MaterialAsset {
            id: AssetIndex::BundleTypeIndex(self.id.clone(), BundleAssetType::Material, index),
            name: Some(material.name.clone()),
            alpha_mode: MaterialAlphaMode::Opaque,
            data: MaterialAssetData::BlinnPhong {
                ambient_color,
                diffuse_color,
                specular_color,
                shininess,
                dissolve,
                optical_density,
                ambient_texture: ambient_texture.map(TextureInfo::from_texture),
                diffuse_texture: diffuse_texture.map(TextureInfo::from_texture),
                specular_texture: specular_texture.map(TextureInfo::from_texture),
                shininess_texture: shininess_texture.map(TextureInfo::from_texture),
                dissolve_texture: dissolve_texture.map(TextureInfo::from_texture),
            },
            normal_texture: normal_texture.map(NormalTextureInfo::from_texture),
            occlusion_texture: None,
            emissive_texture: None,
            emissive_factor: [0.0, 0.0, 0.0],
            double_sided: false,
            uv_animation: None,
        });
        self.material_cache.insert(index, material.clone());
        Ok(Some(material))
    }

    pub fn load_model(&mut self, model: &Model) -> Result<PrimitiveAsset, ObjLoadError<A::Error>> {
        let mesh = &model.mesh;

        let position = chunk_vec3(&mesh.positions);
        let tangent = calculate_tangent(
            PrimitiveAssetMode::TriangleList,
            &position,
            Some(&mesh.indices),
        );
        let normal: Vec<[f32; 3]> = if !mesh.normals.is_empty() {
            chunk_vec3(&mesh.normals)
        } else {
            tangent.clone()
        };
        let tangent = pad_vec3_to_vec4(&tangent, 1.0);
        let tex_coord = if !mesh.texcoords.is_empty() {
            let tex_coord = mesh
                .texcoords
                .chunks_exact(2)
                .map(|chunk| [chunk[0], 1.0 - chunk[1]])
                .collect();
            Some(tex_coord)
        } else {
            None
        };
        let color = if !mesh.vertex_color.is_empty() {
            let color = mesh
                .vertex_color
                .chunks_exact(3)
                .map(|chunk| [chunk[0], chunk[1], chunk[2], 1.0])
                .collect();
            Some(color)
        } else {
            None
        };

        let material = mesh
            .material_id
            .map(|material| self.load_material(material))
            .transpose()?
            .flatten();

        Ok(PrimitiveAsset {
            attributes: PrimitiveAssetAttributes {
                position,
                normal,
                tangent,
                tex_coord: tex_coord
                    .map(|tex_coord| vec![tex_coord])
                    .unwrap_or_default(),
                color: color.map(|color| vec![color]).unwrap_or_default(),
                joints: vec![],
                weights: vec![],
            },
            indices: None,
            material,
            mode: PrimitiveAssetMode::TriangleList,
            targets: vec![],
        })
    }
}

pub fn load_bundle<T, A: Archive<T>>(
    id: BundleIndex,
    bundle: &mut A,
    params: &AssetLoadParams,
) -> Result<MeshAsset, ObjLoadError<A::Error>> {
    let file_name = params.bundle_model_filename("pmx");

    let mut file_entry = bundle
        .by_path(&file_name)
        .map_err(ObjLoadError::Io)?
        .ok_or_else(|| ObjLoadError::ModelNotFound(file_name))?;
    let file = file_entry.unpack().map_err(ObjLoadError::Io)?;
    drop(file_entry);

    let bundle = RefCell::new(bundle);

    let (model, materials) = tobj::load_obj_buf(
        &mut Cursor::new(file),
        &LoadOptions {
            single_index: true,
            triangulate: true,
            ..Default::default()
        },
        |path| {
            let mut bundle = bundle.borrow_mut();
            let mut file_entry = bundle
                .by_path(path)
                .map_err(|_| LoadError::OpenFileFailed)?
                .ok_or(LoadError::OpenFileFailed)?;
            let buffer = file_entry.unpack().map_err(|_| LoadError::OpenFileFailed)?;
            tobj::load_mtl_buf(&mut Cursor::new(buffer))
        },
    )?;
    let materials = materials?;

    let bundle = bundle.into_inner();
    let mut loader = ObjLoader::new(id, bundle, &materials);

    let primitives = model
        .into_iter()
        .map(|model| loader.load_model(&model))
        .collect::<Result<_, _>>()?;
    Ok(MeshAsset {
        name: None,
        primitives,
        weights: vec![],
    })
}
