use glam::EulerRot;
use wgpu::{Device, Queue};

use crate::renderer::{
    camera::{Camera, CameraProjection, CameraView},
    context::Context,
    RendererState,
};

use super::{new_node_id, RenderNode};

#[derive(Debug, Clone)]
pub struct CameraNode {
    id: usize,
    label: Option<String>,
    projection: CameraProjection,
    view: Option<CameraView>,
    updated: bool,
    enabled: bool,
}

impl RenderNode for CameraNode {
    fn id(&self) -> usize {
        self.id
    }

    fn update(&mut self, context: &Context, invalid: bool) -> bool {
        if invalid || self.view.is_none() {
            let (_, rotation, translation) = context.transform().to_scale_rotation_translation();
            let (_, angle_y, angle_z) = rotation.to_euler(EulerRot::XYZ);
            let view = Some(CameraView {
                eye: translation,
                yaw: angle_y.to_degrees() - 90.0,
                pitch: angle_z.to_degrees(),
            });
            self.view = view;

            self.updated = true;
        }
        false
    }

    fn prepare(&mut self, _device: &Device, _queue: &Queue, renderer_state: &mut RendererState) {
        if renderer_state.enabled_camera() == Some(self.id) {
            if let Some(view) = &self.view {
                if self.updated || !self.enabled {
                    let camera = Camera::new(view.clone(), self.projection.clone());
                    renderer_state.set_enabled_camera_data(camera);
                    self.updated = false;
                    self.enabled = true;
                }
            }
        } else {
            self.enabled = false;
        }
    }
}

impl CameraNode {
    pub fn new(projection: CameraProjection, label: Option<String>) -> Self {
        Self {
            id: new_node_id(),
            label,
            projection,
            view: None,
            updated: false,
            enabled: false,
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn projection(&self) -> &CameraProjection {
        &self.projection
    }
}
