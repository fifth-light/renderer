use std::{
    fmt::{Display, Formatter},
    fs::File,
    io,
    path::Path,
    sync::Arc,
};

use binrw::BinRead;
use format::{PmxFile, PmxMaterial, PmxTexture};

use crate::asset::{
    material::MaterialAsset,
    mesh::MeshAsset,
    node::NodeAsset,
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

#[derive(Default)]
pub struct PmxLoader {
    texture_loader: TextureLoader,
}

impl PmxLoader {
    pub fn load_texture(
        &mut self,
        base_path: &Path,
        texture: &PmxTexture,
    ) -> Result<Arc<TextureAsset>, PmxLoadError> {
        let path = base_path.join(&texture.path);
        let id = TextureAssetId::from_path(&path);
        let texture_asset =
            self.texture_loader
                .load_from_path(id, &path, SamplerAsset::default())?;
        Ok(texture_asset)
    }

    pub fn load_material(
        &mut self,
        base_path: &Path,
        file: &PmxFile,
        material: &PmxMaterial,
    ) -> Result<MaterialAsset, PmxLoadError> {
        let texture = match material.texture_index.0 {
            Some(index) => Some(self.load_texture(base_path, &file.textures[index])?),
            None => None,
        };
        Ok(MaterialAsset {
            name: Some(material.material_name_local.clone()),
            diffuse_color: Some(material.diffuse_color),
            diffuse_texture: texture,
            alpha_mode: None,
        })
    }

    pub fn load_surfaces(
        &mut self,
        base_path: &Path,
        path: &Path,
        file: &PmxFile,
    ) -> Result<Vec<NodeAsset>, PmxLoadError> {
        let mut surfaces_next = file.surfaces.as_slice();
        let mut nodes = Vec::new();
        for (index, material) in file.materials.iter().enumerate() {
            let material_asset = self.load_material(base_path, file, material)?;
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
                id: (path.to_owned().to_path_buf(), index).into(),
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

    pub fn load_file(
        &mut self,
        base_path: &Path,
        path: &Path,
        file: &PmxFile,
    ) -> Result<SceneAsset, PmxLoadError> {
        let surfaces = self.load_surfaces(base_path, path, file)?;
        Ok(SceneAsset {
            name: None,
            nodes: surfaces,
            ..Default::default()
        })
    }

    pub fn load(&mut self, base_path: &Path, path: &Path) -> Result<SceneAsset, PmxLoadError> {
        let mut file = File::open(path)?;
        let file = PmxFile::read_le(&mut file)?;
        self.load_file(base_path, path, &file)
    }
}
