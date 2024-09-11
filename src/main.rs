use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_rapier3d::prelude::*;
use bevy_tnua::builtins::TnuaBuiltinDash;
use bevy_tnua::prelude::*;
use bevy_tnua_rapier3d::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            RapierPhysicsPlugin::<NoUserData>::default(),
            TnuaControllerPlugin::default(),
            TnuaRapier3dPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (player_movement, player_aim, jetpack_recharge).in_set(TnuaUserControlsSystemSet),
        )
        .add_systems(Update, (spawn_bullets, update_bullets))
        .run();
}

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Jetpack {
    fuel: f32,
    max_fuel: f32,
    is_active: bool,
    last_used: f32,
}

#[derive(Component)]
pub struct Bullet {
    pub lifetime: Timer,
    pub speed: f32,
    pub direction: Vec3,
}

impl Default for Jetpack {
    fn default() -> Self {
        Self {
            fuel: 2.0,
            max_fuel: 2.0,
            is_active: false,
            last_used: 0.0,
        }
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 20.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Ground
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::new(
                Vec3::new(0.0, 1.0, 0.0),
                Vec2::new(50.0, 50.0),
            )),
            material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
            ..default()
        },
        RigidBody::Fixed,
        Collider::cuboid(25.0, 0.1, 25.0),
    ));

    // Ambient light
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1000.0,
    });

    // Additional light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 10000.0,
            radius: 100.0,
            range: 100.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 8.0, 0.0),
        ..default()
    });

    // Obstacles
    let obstacle_positions = [
        (Vec3::new(5.0, 0.5, 5.0), Vec3::new(2.0, 1.0, 2.0)),
        (Vec3::new(-5.0, 1.0, -5.0), Vec3::new(3.0, 2.0, 3.0)),
        (Vec3::new(0.0, 0.75, -8.0), Vec3::new(4.0, 1.5, 2.0)),
        (Vec3::new(8.0, 0.5, 0.0), Vec3::new(2.0, 1.0, 4.0)),
    ];
    let obstacle_colors = [
        Color::rgb(0.8, 0.2, 0.2),
        Color::rgb(0.2, 0.8, 0.2),
        Color::rgb(0.2, 0.2, 0.8),
        Color::rgb(0.8, 0.8, 0.2),
    ];

    for ((position, size), color) in obstacle_positions.iter().zip(obstacle_colors.iter()) {
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(size.x, size.y, size.z)),
                material: materials.add(*color),
                transform: Transform::from_translation(*position),
                ..default()
            },
            RigidBody::Fixed,
            Collider::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0),
        ));
    }

    // Player
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::rgb(0.8, 0.2, 0.3)),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        Player,
        RigidBody::Dynamic,
        Collider::cylinder(0.5, 0.5),
        TnuaControllerBundle::default(),
        TnuaRapier3dIOBundle::default(),
        LockedAxes::ROTATION_LOCKED,
        Jetpack::default(),
    ));
}

fn player_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut TnuaController, &mut Jetpack), With<Player>>,
) {
    let mut direction = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::KeyW) {
        direction.z -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) {
        direction.z += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if let Ok((mut controller, mut jetpack)) = query.get_single_mut() {
        let jetpack_active = keyboard_input.pressed(KeyCode::Space) && jetpack.fuel > 0.0;

        if jetpack_active {
            jetpack.is_active = true;
            jetpack.fuel = (jetpack.fuel - time.delta_seconds()).max(0.0);
            jetpack.last_used = time.elapsed_seconds();

            let jetpack_direction = if direction != Vec3::ZERO {
                direction.normalize()
            } else {
                Vec3::Y
            };

            controller.action(TnuaBuiltinDash {
                allow_in_air: true,
                desired_forward: jetpack_direction,
                speed: 10.0,
                displacement: jetpack_direction * 5.0,
                ..default()
            });
        } else {
            jetpack.is_active = false;
            controller.basis(TnuaBuiltinWalk {
                desired_velocity: direction.normalize_or_zero() * 5.0,
                float_height: 1.0,
                ..default()
            });
        }
    }
}

fn player_aim(
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut player_query: Query<&mut Transform, With<Player>>,
) {
    let (camera, camera_transform) = camera_query.single();
    let window = windows.single();

    if let Some(cursor_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
        .map(|ray| {
            ray.origin + ray.direction * (camera_transform.translation().y / ray.direction.y)
        })
    {
        if let Ok(mut player_transform) = player_query.get_single_mut() {
            let mut direction = (cursor_position - player_transform.translation).normalize();
            direction.y = 0.0; // Lock the direction to the XZ plane
            direction = direction.normalize(); // Re-normalize after removing Y component
            let current_position = player_transform.translation;
            player_transform.look_at(current_position + direction, Vec3::Y);
        }
    }
}

fn jetpack_recharge(time: Res<Time>, mut query: Query<&mut Jetpack>) {
    for mut jetpack in query.iter_mut() {
        if !jetpack.is_active && time.elapsed_seconds() - jetpack.last_used > 2.0 {
            jetpack.fuel = (jetpack.fuel + time.delta_seconds()).min(jetpack.max_fuel);
        }
    }
}
fn spawn_bullets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    player_query: Query<&Transform, With<Player>>,
) {
    let (camera, camera_transform) = camera_query.single();
    let player_transform = player_query.single();
    let window = windows.single();

    if mouse_button_input.just_pressed(MouseButton::Left) {
        if let Some(world_pos) = window
            .cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
            .map(|ray| {
                ray.origin + ray.direction * (camera_transform.translation().y / ray.direction.y)
            })
        {
            let mut direction = (world_pos - player_transform.translation).normalize();
            direction.y = 0.0; // Lock the direction to the XZ plane
            direction = direction.normalize(); // Re-normalize after removing Y component

            println!(
                "Spawning bullet at: {:?}, direction: {:?}",
                player_transform.translation, direction
            );

            commands.spawn((
                MaterialMeshBundle {
                    mesh: meshes.add(Mesh::from(Sphere {
                        radius: 0.1, // Adjust this value to change bullet size
                    })),
                    material: materials.add(StandardMaterial {
                        base_color: LinearRgba::RED.into(),
                        ..default()
                    }),
                    transform: Transform::from_translation(player_transform.translation)
                        .looking_at(player_transform.translation + direction, Vec3::Y),
                    ..default()
                },
                Bullet {
                    lifetime: Timer::from_seconds(3.0, TimerMode::Once),
                    speed: 20.0,
                    direction,
                },
            ));
        }
    }
}

fn update_bullets(
    mut commands: Commands,
    mut bullet_query: Query<(Entity, &mut Transform, &mut Bullet)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut bullet) in bullet_query.iter_mut() {
        bullet.lifetime.tick(time.delta());

        if bullet.lifetime.finished() {
            commands.entity(entity).despawn();
        } else {
            let movement = transform.forward() * bullet.speed * time.delta_seconds();
            transform.translation += movement;
        }
    }
}
