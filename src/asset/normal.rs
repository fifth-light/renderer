use glam::Vec3;

use super::primitive::PrimitiveAssetMode;

fn calculate_triangle_normal(positions: &[&[f32; 3]; 3]) -> Vec3 {
    let pnt_0 = Vec3::from_array(*positions[0]);
    let pnt_1 = Vec3::from_array(*positions[1]);
    let pnt_2 = Vec3::from_array(*positions[2]);
    let vec_a = pnt_2 - pnt_1;
    let vec_b = pnt_0 - pnt_1;
    vec_a.cross(vec_b).normalize()
}

pub fn calculate_normal(
    mode: PrimitiveAssetMode,
    positions: &[[f32; 3]],
    indices: Option<&[u32]>,
) -> Vec<[f32; 3]> {
    let mut buffer = vec![Vec3::ZERO; positions.len()];
    match mode {
        // Points and lines don't have a normal for now
        PrimitiveAssetMode::Points
        | PrimitiveAssetMode::LineStrip
        | PrimitiveAssetMode::LineList => {}
        PrimitiveAssetMode::TriangleStrip => todo!(),
        PrimitiveAssetMode::TriangleList => {
            if let Some(indices) = indices {
                let triangles = indices.len() / 3;
                (0..triangles).for_each(|triangle_index| {
                    let indices: Vec<usize> = (triangle_index * 3..triangle_index * 3 + 3)
                        .map(|index| indices[index] as usize)
                        .collect();
                    let positions: Vec<&[f32; 3]> =
                        indices.iter().map(|index| &positions[*index]).collect();
                    let positions: [&[f32; 3]; 3] = positions.try_into().unwrap();
                    let normal = calculate_triangle_normal(&positions);
                    indices.iter().for_each(|index| buffer[*index] += normal);
                });
            } else {
                let triangles = positions.len() / 3;
                (0..triangles).for_each(|triangle_index| {
                    let indices: Vec<usize> =
                        (triangle_index * 3..triangle_index * 3 + 3).collect();
                    let positions: Vec<&[f32; 3]> =
                        indices.iter().map(|index| &positions[*index]).collect();
                    let positions: [&[f32; 3]; 3] = positions.try_into().unwrap();
                    let normal = calculate_triangle_normal(&positions);
                    indices.iter().for_each(|index| buffer[*index] += normal);
                });
            }
        }
    };
    for point_normal in &mut buffer {
        *point_normal = point_normal.normalize();
    }
    buffer.into_iter().map(|normal| normal.to_array()).collect()
}
