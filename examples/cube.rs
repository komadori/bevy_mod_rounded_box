use std::f32::consts::TAU;

use bevy::{prelude::*, render::mesh::VertexAttributeValues};

use bevy_mod_rounded_box::*;

#[bevy_main]
fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(bevy::window::close_on_esc)
        .add_system(rotate_cube)
        .run();
}

#[derive(Component)]
struct TheCube;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Generate mesh
    let mut mesh = Mesh::from(RoundedBox {
        size: Vec3::new(2., 2., 2.),
        radius: 0.4,
        subdivisions: 4,
        options: BoxMeshOptions::DEFAULT.with_uv().with_face(),
    });

    // Remap faces to colours
    if let Some(VertexAttributeValues::Uint32(faces)) = mesh.remove_attribute(ATTRIBUTE_FACE) {
        const COLOUR_MAP: [[f32; 4]; 6] = [
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 1.0],
            [1.0, 0.4, 0.4, 1.0],
            [0.4, 1.0, 0.4, 1.0],
            [0.4, 0.4, 1.0, 1.0],
        ];
        let mut colours = Vec::with_capacity(faces.len());
        for face in faces {
            colours.push(COLOUR_MAP[face as usize]);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    }

    // Spawn cube et al.
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(bevy::prelude::shape::Plane { size: 10.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        transform: Transform::from_xyz(0.0, -2.0, 0.0),
        ..default()
    });
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(mesh),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(asset_server.load("not_mirrored.png")),
                ..Default::default()
            }),
            ..default()
        })
        .insert(TheCube);
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn rotate_cube(
    mut cubes: Query<&mut Transform, With<TheCube>>,
    timer: Res<Time>,
    mut t: Local<f32>,
) {
    let ta = *t;
    *t = (ta + 0.5 * timer.delta_seconds()) % TAU;
    let tb = *t;
    let i1 = tb.cos() - ta.cos();
    let i2 = ta.sin() - tb.sin();
    for mut transform in cubes.iter_mut() {
        transform.rotate(Quat::from_rotation_z(
            TAU * 20.0 * i1 * timer.delta_seconds(),
        ));
        transform.rotate(Quat::from_rotation_y(
            TAU * 20.0 * i2 * timer.delta_seconds(),
        ));
    }
}
