use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum PointCloudBlend {
    #[default]
    Additive,
    Alpha,
    Opaque,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Reflect)]
pub enum PointCloudShape {
    #[default]
    Circle,
    Square,
}

/// Billboard quad mesh (6 verts, 2 tris) used as the instanced template.
pub fn make_point_cloud_mesh() -> Mesh {
    let positions: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; 6];
    let normals = vec![[0.0_f32, 1.0, 0.0]; 6];
    let uvs = vec![[0.0_f32, 0.0]; 6];
    let indices: Vec<u32> = vec![0, 1, 2, 3, 4, 5];

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_cloud_mesh_has_correct_structure() {
        let mesh = make_point_cloud_mesh();
        assert_eq!(mesh.count_vertices(), 6);

        let Indices::U32(indices) = mesh.indices().unwrap() else {
            panic!("expected U32 indices");
        };
        assert_eq!(indices, &[0, 1, 2, 3, 4, 5]);

        assert!(mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());
        assert!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some());
    }
}
