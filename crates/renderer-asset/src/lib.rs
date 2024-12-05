//! Provide asset handling for renderer.
//!
//! This library provides node structure which is structured upon GLTF,
//! and loaders for multiple formats such as GLTF, OBJ and PMX. This
//! library also provides archive processor to isolate resource loading
//! from file-system, allowing model and resource to be packed into an
//! archive file to be loaded at once.
//!
pub mod animation;
pub mod archive;
pub mod camera;
pub mod index;
/// Model loaders for various formats
pub mod loader;
pub mod material;
pub mod mesh;
pub mod node;
pub mod primitive;
pub mod scene;
pub mod skin;
pub mod tangent;
pub mod texture;
