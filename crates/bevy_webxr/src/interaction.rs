use crate::{XrFrom, XrInto};
use bevy_math::{Mat4, Quat, Vec2, Vec3};


impl XrFrom<web_sys::XrRigidTransform> for bevy_xr::interaction::XrRigidTransform {
    fn xr_from(rigid_transform: web_sys::XrRigidTransform) -> Self {
        let position = rigid_transform.position();
        let orientation = rigid_transform.orientation();

        bevy_xr::interaction::XrRigidTransform {
            position: Vec3::new(position.x(), position.y(), position.z()),
            orientation: Quat::from_xyzw(orientation.x(), orientation.y(), orientation.z(), orientation.w()),
        } 
    }
}

impl XrFrom<bevy_xr::interaction::XrRigidTransform> for web_sys::XrRigidTransform {
    fn xr_from(rigid_transform: bevy_xr::interaction::XrRigidTransform) -> Self {
        let position = web_sys::DomPointInit::new();
        dom_point.x(rigid_transform.position.x);
        dom_point.y(rigid_transform.position.y);
        dom_point.z(rigid_transform.position.z);

        let quat_array = rigid_transform.orientation.to_array();
        let orientation = web_sys::DomPointInit::new();
        dom_point.x(quat_array[0]);
        dom_point.y(quat_array[1]);
        dom_point.z(quat_array[2]);
        dom_point.w(quat_array[3]);

        web_sys::XrRigidTransform::new_with_position_and_orientation(
            &position, 
            &orientation,
        ).expect("Failed to cast from bevy_xr::XrRigidTransform to web_sys::XrRigidTransform")
    }
}

impl XrFrom<web_sys::XrPose> for bevy_xr::interaction::XrPose {
    fn xr_from(pose: web_sys::XrPose) -> Self {
        bevy_xr::interaction::XrPose {
            transform: pose.transform().xr_into(),
            linear_velocity: pose.linear_velocity().map(|point| Vec3::new(point.x(), point.y(), point.z())),
            angular_velocity: pose.angular_velocity().map(|point| Vec3::new(point.x(), point.y(), point.z())),
            emulated_position: pose.emulated_position(),
        }
    }
}

