use bevy::prelude::*;
use bevy_fps_controller::controller::*;
use bevy_mod_picking::backends::raycast::RaycastBackend;
use bevy_mod_picking::picking_core::PickingPluginsSettings;
use bevy_mod_picking::prelude::*;
use bevy_rapier3d::prelude::*;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugins(FpsControllerPlugin)
        .add_plugins(RapierDebugRenderPlugin::default())
        .insert_resource(PickingPluginsSettings {
            is_input_enabled: true,
            is_focus_enabled: true,
            ..default()
        })
        .add_plugins(DefaultPickingPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (change_object_color, update_fps_camera))
        .run();
}

#[derive(Component)]
struct ClickableObject;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
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

    // Floor
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(50.0, 50.0)),
            material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Collider::cuboid(25.0, 0.1, 25.0), // Add a collider to the floor
        PickableBundle::default(),
    ));

    // Clickable objects
    for i in 0..5 {
        let position = Vec3::new(i as f32 * 2.0 - 4.0, 0.5, 0.0);
        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
                material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
                transform: Transform::from_translation(position),
                ..default()
            },
            Collider::cuboid(0.5, 0.5, 0.5), // Add colliders to the objects
            PickableBundle::default(),
            ClickableObject,
        ));
    }

    // FPS Controller
    let logical_entity = commands
        .spawn((
            TransformBundle::from_transform(Transform::from_xyz(0.0, 5.0, 5.0)),
            Collider::cylinder(3.0 / 2.0, 0.5),
            ActiveEvents::COLLISION_EVENTS,
            LockedAxes::ROTATION_LOCKED,
            AdditionalMassProperties::Mass(1.0),
            Velocity::zero(),
            RigidBody::Dynamic,
            Ccd { enabled: true },
            GravityScale(1.0), // Enable gravity
            Friction {
                coefficient: 0.5,
                combine_rule: CoefficientCombineRule::Average,
            },
            Restitution {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            LogicalPlayer,
            FpsControllerInput {
                pitch: -std::f32::consts::FRAC_PI_8,
                yaw: std::f32::consts::FRAC_PI_4,
                ..default()
            },
            FpsController {
                walk_speed: 10.0,
                run_speed: 20.0,
                jump_speed: 6.0,
                //gravity: -9.81,
                ..default()
            },
        ))
        .insert(CameraConfig {
            height_offset: 0.6, // Adjust this to change the camera height
        })
        .id();

    // Camera
    commands.spawn((Camera3dBundle::default(), RenderPlayer { logical_entity }));
}

fn change_object_color(
    mut events: EventReader<Pointer<Click>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut clickable_objects: Query<(&Handle<StandardMaterial>, &mut ClickableObject)>,
) {
    let mut rng = rand::thread_rng();
    for event in events.read() {
        if let Ok((material_handle, _)) = clickable_objects.get_mut(event.target) {
            if let Some(material) = materials.get_mut(material_handle) {
                material.base_color = Color::srgb(
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                    rng.gen_range(0.0..1.0),
                );
            }
        }
    }
}

fn update_fps_camera(
    logical_query: Query<(&Transform, &FpsController), With<LogicalPlayer>>,
    mut render_query: Query<&mut CameraConfig, With<RenderPlayer>>,
) {
    if let (Ok((logical_transform, fps_controller)), Ok(mut camera_config)) =
        (logical_query.get_single(), render_query.get_single_mut())
    {
        camera_config.height_offset = fps_controller.height;
    }
}
