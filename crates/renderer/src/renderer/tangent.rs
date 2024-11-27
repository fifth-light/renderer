use std::{cmp::Ordering, collections::BTreeMap, ops::Deref};

use glam::Vec3;

use crate::asset::{normal::calculate_normal, primitive::PrimitiveAssetMode};

struct PositionFloat {
    value: f32,
}

impl PositionFloat {
    fn new(value: f32) -> Self {
        Self { value }
    }
}

impl Deref for PositionFloat {
    type Target = f32;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

const FLOAT_COMPARE_LIMIT: f32 = 0.00001;

impl PartialEq for PositionFloat {
    fn eq(&self, other: &Self) -> bool {
        (self.value - other.value).abs() < FLOAT_COMPARE_LIMIT
    }
}

impl Eq for PositionFloat {}

impl PartialOrd for PositionFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PositionFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.eq(other) {
            Ordering::Equal
        } else {
            self.value.total_cmp(&other.value)
        }
    }
}

impl From<f32> for PositionFloat {
    fn from(value: f32) -> Self {
        PositionFloat::new(value)
    }
}

fn f32_array_to_position(array: &[f32; 3]) -> [PositionFloat; 3] {
    [array[0].into(), array[1].into(), array[2].into()]
}

pub fn _calculate_tangent(
    mode: PrimitiveAssetMode,
    positions: &[[f32; 3]],
    indices: Option<&[u32]>,
) -> Vec<[f32; 3]> {
    let normals = calculate_normal(mode, positions, indices);
    let mut points = BTreeMap::new();
    for (index, position) in positions.iter().enumerate() {
        let normal = Vec3::from_array(normals[index]);
        let position = f32_array_to_position(position);
        points
            .entry(position)
            .and_modify(|item| *item += normal)
            .or_insert(normal);
    }
    for point in points.values_mut() {
        *point = point.normalize();
    }
    positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let position = f32_array_to_position(position);
            points
                .get(&position)
                .map(|item| item.to_array())
                .unwrap_or_else(|| normals[index])
        })
        .collect()
}
