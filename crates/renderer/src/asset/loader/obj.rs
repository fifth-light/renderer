use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    path::Path,
};

use tobj::{LoadError, LoadOptions};

use crate::asset::{
    material::MaterialAsset,
    mesh::MeshAsset,
    normal::calculate_normal,
    primitive::{PrimitiveAsset, PrimitiveAssetMode},
    texture::{SamplerAsset, TextureAssetId},
};

use super::{
    chunk_vec3, pad_color_vec3_to_vec4,
    texture::{TextureLoadError, TextureLoader},
};

#[derive(Debug)]
pub enum ObjLoadError {
    Obj(LoadError),
    Texture(TextureLoadError),
}

impl Display for ObjLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ObjLoadError::Obj(err) => Display::fmt(&err, f),
            ObjLoadError::Texture(err) => Display::fmt(&err, f),
        }
    }
}

impl Error for ObjLoadError {}

impl From<LoadError> for ObjLoadError {
    fn from(value: LoadError) -> Self {
        ObjLoadError::Obj(value)
    }
}

impl From<TextureLoadError> for ObjLoadError {
    fn from(value: TextureLoadError) -> Self {
        ObjLoadError::Texture(value)
    }
}

#[derive(Default)]
pub struct ObjLoader {
    texture_loader: TextureLoader,
}

impl ObjLoader {
    pub fn load(&mut self, base_path: &Path, path: &Path) -> Result<MeshAsset, ObjLoadError> {
        let (models, materials) = tobj::load_obj(
            path,
            &LoadOptions {
                single_index: true,
                triangulate: true,
                ..Default::default()
            },
        )?;
        let materials = materials?;

        let primitives = models
            .into_iter()
            .map(|model| {
                let mesh = model.mesh;

                let positions = chunk_vec3(mesh.positions);
                let normals: Vec<[f32; 3]> = if !mesh.normals.is_empty() {
                    chunk_vec3(mesh.normals)
                } else {
                    calculate_normal(
                        PrimitiveAssetMode::TriangleList,
                        &positions,
                        Some(&mesh.indices),
                    )
                };
                let tex_coords = if !mesh.texcoords.is_empty() {
                    let tex_coords = mesh
                        .texcoords
                        .chunks_exact(2)
                        .map(|chunk| [chunk[0], 1.0 - chunk[1]])
                        .collect();
                    Some(tex_coords)
                } else {
                    None
                };
                let vertex_color = if !mesh.vertex_color.is_empty() {
                    let vertex_color = mesh
                        .vertex_color
                        .chunks_exact(3)
                        .map(|chunk| [chunk[0], chunk[1], chunk[2], 1.0])
                        .collect();
                    Some(vertex_color)
                } else {
                    None
                };

                let material = mesh
                    .material_id
                    .and_then(|material| materials.get(material));
                let material = if let Some(material) = material {
                    let diffuse_texture = if let Some(diffuse_texture) = &material.diffuse_texture {
                        let path = base_path.join(diffuse_texture);
                        let diffuse_texture = self.texture_loader.load_from_path(
                            TextureAssetId::from_path(&path),
                            &path,
                            SamplerAsset::default(),
                        )?;
                        Some(diffuse_texture)
                    } else {
                        None
                    };

                    let material = MaterialAsset {
                        name: None,
                        unlit: false,
                        diffuse_color: material.diffuse.map(pad_color_vec3_to_vec4),
                        diffuse_texture,
                        alpha_mode: None,
                    };
                    Some(material)
                } else {
                    None
                };

                Ok(PrimitiveAsset {
                    name: Some(model.name),
                    positions,
                    normals,
                    tex_coords: tex_coords.into_iter().collect(),
                    vertex_color: vertex_color.into_iter().collect(),
                    indices: Some(mesh.indices),
                    skin: vec![],
                    material,
                    mode: PrimitiveAssetMode::TriangleList,
                })
            })
            .collect::<Result<_, ObjLoadError>>()?;

        Ok(MeshAsset {
            name: Some(path.to_string_lossy().to_string()),
            primitives,
        })
    }
}
