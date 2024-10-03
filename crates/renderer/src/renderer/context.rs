use std::collections::BTreeMap;

use glam::Mat4;
use log::warn;

use super::node::light::LightData;

pub static DEFAULT_LOCAL_CONTEXT: LocalContext = LocalContext {
    transform: Mat4::IDENTITY,
};

#[derive(Debug, Clone)]
pub struct LocalContext {
    transform: Mat4,
}

impl Default for LocalContext {
    fn default() -> Self {
        DEFAULT_LOCAL_CONTEXT.clone()
    }
}

impl LocalContext {
    pub fn transform(&self) -> &Mat4 {
        &self.transform
    }

    pub fn add_transform(&self, transform: &Mat4) -> Self {
        Self {
            transform: self.transform * (*transform),
        }
    }
}

#[derive(Default)]
pub struct GlobalContext {
    updated_joints: BTreeMap<usize, BTreeMap<usize, Mat4>>,
    lights: Vec<LightData>,
}

impl GlobalContext {
    pub fn update_joint(&mut self, skin: usize, joint_index: usize, matrix: Mat4) {
        let skin_map = self.updated_joints.entry(skin).or_default();
        if skin_map.insert(joint_index, matrix).is_some() {
            warn!(
                "Joint #{} of skin #{} is already set in global context",
                joint_index, skin
            );
        }
    }

    pub fn updated_joints(&self) -> &BTreeMap<usize, BTreeMap<usize, Mat4>> {
        &self.updated_joints
    }

    pub fn add_light(&mut self, data: LightData) {
        self.lights.push(data);
    }

    pub fn finish(self) -> Vec<LightData> {
        self.lights
    }
}
