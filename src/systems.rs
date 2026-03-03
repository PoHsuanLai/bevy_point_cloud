use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;

use crate::material::make_point_cloud_mesh;
use crate::point_cloud::PointCloud;

pub fn init_point_clouds(
    mut commands: Commands,
    query: Query<(Entity, Has<Mesh3d>), Added<PointCloud>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, has_mesh) in &query {
        let mut cmds = commands.entity(entity);

        if !has_mesh {
            let mesh = meshes.add(make_point_cloud_mesh());
            cmds.insert(Mesh3d(mesh));
        }

        cmds.insert((Visibility::default(), NoFrustumCulling));
    }
}
