#[derive(Debug, Clone)]
pub struct CameraAsset {
    pub projection: CameraProjectionAsset,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CameraProjectionAsset {
    Orthographic(OrthographicCameraAsset),
    Perspective(PerspectiveCameraAsset),
}

#[derive(Debug, Clone)]
pub struct OrthographicCameraAsset {
    pub xmag: f32,
    pub ymag: f32,
    pub zfar: f32,
    pub znear: f32,
}

#[derive(Debug, Clone)]
pub struct PerspectiveCameraAsset {
    pub aspect_radio: Option<f32>,
    pub yfov: f32,
    pub zfar: Option<f32>,
    pub znear: f32,
}
