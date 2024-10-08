use std::{
    cell::RefCell,
    collections::BTreeMap,
    fmt::{Display, Formatter},
    fs::File,
    io,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use binrw::BinRead;
use format::{PmxFile, PmxMaterial, PmxTexture};
use glam::Vec3;

use crate::asset::{
    material::MaterialAsset,
    mesh::MeshAsset,
    node::{DecomposedTransform, NodeAsset, NodeAssetId, NodeTransform},
    primitive::{PrimitiveAsset, PrimitiveAssetMode},
    scene::SceneAsset,
    texture::{SamplerAsset, TextureAsset, TextureAssetId},
};

use super::texture::{TextureLoadError, TextureLoader};

mod format;

#[derive(Debug)]
pub enum PmxLoadError {
    Format(binrw::Error),
    Io(io::Error),
    Texture(TextureLoadError),
    NoSurfaceLeft { expected: usize, actual: usize },
    BadSurfacesCount(usize),
}

impl Display for PmxLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PmxLoadError::Format(format) => format.fmt(f),
            PmxLoadError::Io(io) => io.fmt(f),
            PmxLoadError::Texture(texture) => texture.fmt(f),
            PmxLoadError::NoSurfaceLeft { expected, actual } => write!(
                f,
                "Want to read {} surfaces for material, but only {} left",
                expected, actual
            ),
            PmxLoadError::BadSurfacesCount(count) => {
                write!(f, "Bad surfaces count in material: {}", count)
            }
        }
    }
}

impl std::error::Error for PmxLoadError {}

impl From<binrw::Error> for PmxLoadError {
    fn from(value: binrw::Error) -> Self {
        Self::Format(value)
    }
}

impl From<io::Error> for PmxLoadError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<TextureLoadError> for PmxLoadError {
    fn from(value: TextureLoadError) -> Self {
        Self::Texture(value)
    }
}

struct BoneItem {
    id: NodeAssetId,
    name: String,
    transform: Vec3,
    children: Vec<Rc<RefCell<BoneItem>>>,
}

struct PmxLoader<'a> {
    texture_loader: TextureLoader,
    path: &'a Path,
    base_path: &'a Path,
}

impl<'a> PmxLoader<'a> {
    fn new(base_path: &'a Path, path: &'a Path) -> Self {
        Self {
            texture_loader: TextureLoader::default(),
            path,
            base_path,
        }
    }

    fn path(&self) -> PathBuf {
        self.path.to_path_buf()
    }

    fn load_texture(&mut self, texture: &PmxTexture) -> Result<Arc<TextureAsset>, PmxLoadError> {
        let path = self.base_path.join(&texture.path);
        let id = TextureAssetId::from_path(&path);
        let texture_asset =
            self.texture_loader
                .load_from_path(id, &path, SamplerAsset::default())?;
        Ok(texture_asset)
    }

    fn load_material(
        &mut self,
        file: &PmxFile,
        material: &PmxMaterial,
    ) -> Result<MaterialAsset, PmxLoadError> {
        let texture = match material.texture_index.0 {
            Some(index) => Some(self.load_texture(&file.textures[index])?),
            None => None,
        };
        Ok(MaterialAsset {
            name: Some(material.material_name_local.clone()),
            diffuse_color: Some(material.diffuse_color),
            diffuse_texture: texture,
            alpha_mode: None,
        })
    }

    fn bone_id(&self, file: &PmxFile, index: usize) -> NodeAssetId {
        (self.path(), file.surfaces_count as usize + index).into()
    }

    fn load_bones(&self, file: &PmxFile) -> Vec<NodeAsset> {
        let mut bones = BTreeMap::new();
        let mut bone_references = BTreeMap::new();
        for (index, bone) in file.bones.iter().enumerate() {
            let item = BoneItem {
                id: self.bone_id(file, index),
                name: bone.bone_name_local.clone(),
                transform: Vec3::from_array(bone.position),
                children: Vec::new(),
            };
            let item = Rc::new(RefCell::new(item));
            bone_references.insert(index, item.clone());
            bones.insert(index, item);
        }

        for (index, bone) in file.bones.iter().enumerate() {
            if let Some(parent_index) = bone.parent_bone_index.0 {
                let bone = bones[&index].clone();
                let mut parent = (*bone_references[&parent_index]).borrow_mut();
                parent.children.push(bone)
            }
        }

        drop(bone_references);

        fn convert_bone(item: BoneItem) -> NodeAsset {
            NodeAsset {
                id: item.id,
                name: Some(item.name),
                transform: Some(NodeTransform::Decomposed(DecomposedTransform {
                    translation: item.transform,
                    ..Default::default()
                })),
                mesh: None,
                skin: None,
                camera: None,
                has_animation: false,
                children: item
                    .children
                    .into_iter()
                    .map(|item| convert_bone(Rc::into_inner(item).unwrap().into_inner()))
                    .collect(),
            }
        }
        bones
            .into_values()
            .map(|item| convert_bone(Rc::into_inner(item).unwrap().into_inner()))
            .collect()
    }

    fn surface_id(&self, index: usize) -> NodeAssetId {
        (self.path(), index).into()
    }

    fn load_surfaces(&mut self, file: &PmxFile) -> Result<Vec<NodeAsset>, PmxLoadError> {
        let mut surfaces_next = file.surfaces.as_slice();
        let mut nodes = Vec::new();
        for (index, material) in file.materials.iter().enumerate() {
            let material_asset = self.load_material(file, material)?;
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

            let mut positions = Vec::new();
            let mut tex_coords = Vec::new();
            let mut normals = Vec::new();
            for surface in surfaces {
                positions.push(surface.position);
                tex_coords.push(surface.uv);
                normals.push(surface.normal);
            }
            let primitive = PrimitiveAsset {
                name: None,
                positions,
                normals,
                tex_coords: vec![tex_coords],
                vertex_color: Vec::new(),
                indices: None,
                skin: Vec::new(),
                material: Some(material_asset),
                mode: PrimitiveAssetMode::TriangleList,
            };
            let mesh = MeshAsset {
                name: None,
                primitives: vec![primitive],
            };
            let node = NodeAsset {
                id: self.surface_id(index),
                name: None,
                transform: None,
                mesh: Some(mesh),
                skin: None,
                camera: None,
                has_animation: false,
                children: vec![],
            };
            nodes.push(node);
        }

        Ok(nodes)
    }

    fn load_file(&mut self, file: PmxFile) -> Result<SceneAsset, PmxLoadError> {
        let surfaces = self.load_surfaces(&file)?;
        let bones = self.load_bones(&file);
        // TODO: calculate the correct nodes and skinned_nodes
        Ok(SceneAsset {
            name: None,
            nodes: surfaces,
            skinned_nodes: bones,
            ..Default::default()
        })
    }
}

pub fn load_pmx(base_path: &Path, path: &Path) -> Result<SceneAsset, PmxLoadError> {
    let mut loader = PmxLoader::new(base_path, path);
    let mut file = File::open(path)?;
    let file = PmxFile::read_le(&mut file)?;
    loader.load_file(file)
}
