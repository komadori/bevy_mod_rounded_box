use std::f32::consts::TAU;

use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

use bevy_mod_rounded_box::*;

#[bevy_main]
fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(rotate_cube)
        .run();
}

#[derive(Component)]
struct TheCube();

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Generate draughts board texture
    let tex_extents = Extent3d {
        width: 256,
        height: 256,
        depth_or_array_layers: 1,
    };
    let mut tex_data = Vec::with_capacity(tex_extents.width as usize * tex_extents.height as usize);
    for i in 0..256 {
        for j in 0..256 {
            let value: f32 = if (i / 16) % 2 ^ (j / 16) % 2 == 0 {
                1.0
            } else {
                0.0
            };
            tex_data.extend_from_slice(&value.to_ne_bytes())
        }
    }
    let image = Image::new(
        tex_extents,
        TextureDimension::D2,
        tex_data,
        TextureFormat::R32Float,
    );

    // Spawn cube et al.
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(bevy::prelude::shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(RoundedBox {
                size: Vec3::new(1., 1., 1.),
                radius: 0.3,
                subdivisions: 3,
            })),
            material: materials.add(StandardMaterial {
                base_color_texture: Some(images.add(image)),
                ..Default::default()
            }),
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            ..default()
        })
        .insert(TheCube());
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    commands.spawn_bundle(PerspectiveCameraBundle {
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
