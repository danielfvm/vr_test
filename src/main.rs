use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_mod_openxr::add_xr_plugins;
use bevy_mod_xr::session::XrTrackingRoot;
use bevy_rapier3d::prelude::*;
use bevy_tnua::prelude::*;
use bevy_tnua_rapier3d::*;

fn main() {
    App::new()
        .add_plugins(add_xr_plugins(DefaultPlugins))
        .add_plugins(bevy_xr_utils::hand_gizmos::HandGizmosPlugin)
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::default())
        //     .add_plugins(RapierDebugRenderPlugin::default())
        .add_plugins(TnuaControllerPlugin::default())
        .add_plugins(TnuaRapier3dPlugin::default())
        .add_systems(Startup, (setup, cursor_grab))
        .add_systems(Update, apply_controls.in_set(TnuaUserControlsSystemSet))
        .add_systems(Update, mouse_look.in_set(TnuaUserControlsSystemSet))
        .add_systems(Update, move_oxr)
        .insert_resource(MouseSettings {
            sensitivity: 0.04,
            pitch_limit: 90.0,
        })
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // circular base
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Circle::new(4.0)),
            material: materials.add(Color::WHITE),
            transform: Transform::from_rotation(Quat::from_rotation_x(
                -std::f32::consts::FRAC_PI_2,
            )),
            ..default()
        })
        .insert(
            Collider::from_bevy_mesh(
                &Circle::new(4.0).mesh().build(),
                &ComputedColliderShape::TriMesh,
            )
            .unwrap(),
        );

    // cube
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(Color::srgb_u8(124, 144, 255)),
            transform: Transform::from_xyz(0.0, 2.5, 0.0),
            ..default()
        })
        .insert(RigidBody::Dynamic)
        .insert(Collider::cuboid(0.5, 0.5, 0.5))
        .insert(Restitution::coefficient(0.7));

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // player
    /*commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-1.5, 1.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });*/
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 4.0, 2.0),
            ..default()
        })
        .insert(RigidBody::Dynamic)
        .insert(Collider::capsule_y(0.5, 0.5))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 4.0, 2.0)))
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(TnuaControllerBundle::default())
        .insert(TnuaRapier3dSensorShape(Collider::cylinder(0.3, 0.0)))
        .insert(TnuaRapier3dIOBundle::default())
        .insert(CameraControl::default());

    // Lock the cursor to the window
    //commands.insert_resource(CursorGrabMode::Confined);
    //commands.insert_resource(CursorIcon::Default);
}

fn cursor_grab(mut q_windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut primary_window = q_windows.single_mut();

    // if you want to use the cursor, but not let it leave the window,
    // use `Confined` mode:
    primary_window.cursor.grab_mode = CursorGrabMode::Confined;

    // for a game that doesn't use the cursor (like a shooter):
    // use `Locked` mode to keep the cursor in one place
    primary_window.cursor.grab_mode = CursorGrabMode::Locked;

    // also hide the cursor
    primary_window.cursor.visible = false;
}

/* Change the position inside of a system. */
fn move_oxr(
    mut commands: Commands,
    mut query: Query<&mut Transform, With<TnuaController>>,
    mut oxr_root: Query<&mut XrTrackingRoot>,
) {

}

/* Change the position inside of a system. */
fn apply_controls(
    mut query: Query<(&mut Transform, &mut TnuaController)>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let Ok((mut transform, mut controller)) = query.get_single_mut() else {
        return;
    };

    let mut movement = Vec3::ZERO;

    if keyboard.pressed(KeyCode::ArrowUp) || keyboard.pressed(KeyCode::KeyW) {
        movement += Vec3::Z;
    }
    if keyboard.pressed(KeyCode::ArrowDown) || keyboard.pressed(KeyCode::KeyS) {
        movement -= Vec3::Z;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
        movement += Vec3::X;
    }
    if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
        movement -= Vec3::X;
    }

    let forward = transform.forward();
    let forward = forward.with_y(0.0).normalize();

    // Create a quaternion to rotate the movement vector to align with the camera's forward
    let angle = forward.angle_between(Vec3::Z);
    let rotation = if forward.cross(Vec3::Z).y > 0.0 {
        Quat::from_rotation_y(-angle)
    } else {
        Quat::from_rotation_y(angle)
    };

    // Feed the basis every frame. Ev en if the player doesn't move - just use `desired_velocity:
    // Vec3::ZERO`. `TnuaController` starts without a basis, which will make the character collider
    // just fall.
    controller.basis(TnuaBuiltinWalk {
        // The `desired_velocity` determines how the character will move.
        desired_velocity: rotation * movement * 5.0,
        // The `float_height` must be greater (even if by little) from the distance between the
        // character's center and the lowest point of its collider.
        float_height: 1.1,
        // `TnuaBuiltinWalk` has many other fields for customizing the movement - but they have
        // sensible defaults. Refer to the `TnuaBuiltinWalk`'s documentation to learn what they do.
        ..Default::default()
    });

    // Feed the jump action every frame as long as the player holds the jump button. If the player
    // stops holding the jump button, simply stop feeding the action.
    if keyboard.pressed(KeyCode::Space) {
        controller.action(TnuaBuiltinJump {
            // The height is the only mandatory field of the jump button.
            height: 2.0,
            // `TnuaBuiltinJump` also has customization fields with sensible defaults.
            ..Default::default()
        });
    }
}

#[derive(Component, Default)]
struct CameraControl {
    pitch: f32, // Vertical rotation
    yaw: f32,   // Horizontal rotation
}

#[derive(Resource, Default)]
struct MouseSettings {
    sensitivity: f32,
    pitch_limit: f32,
}

fn mouse_look(
    mut query: Query<(&mut CameraControl, &mut Transform)>,
    mut motion_events: EventReader<MouseMotion>,
    settings: Res<MouseSettings>,
    time: Res<Time>,
) {
    for (mut control, mut transform) in query.iter_mut() {
        for event in motion_events.read() {
            // Update yaw and pitch
            control.yaw -= event.delta.x * settings.sensitivity;
            control.pitch = (control.pitch - event.delta.y * settings.sensitivity)
                .clamp(-settings.pitch_limit, settings.pitch_limit);

            // Apply rotation
            transform.rotation = Quat::from_axis_angle(Vec3::Y, control.yaw.to_radians())
                * Quat::from_axis_angle(Vec3::X, control.pitch.to_radians());
        }
    }
}
