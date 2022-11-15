use crate::{WebXrContext, XrFrom};
use bevy_math::{Quat, Vec3};


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
            linear_velocity: pose.linear_velocity().map(|point| Vec3::new(point.x(), point.y(), point.z())), angular_velocity: pose.angular_velocity().map(|point| Vec3::new(point.x(), point.y(), point.z())), emulated_position: pose.emulated_position(),
        }
    }
}

impl XrFrom<web_sys::XrReferenceSpaceType> for bevy_xr::interaction::XrReferenceSpaceType {
    fn xr_from(rf_space_type: web_sys::XrReferenceSpaceType) -> Self {
        match rf_space_type {
            web_sys::XrReferenceSpaceType::View => bevy_xr::interaction::XrReferenceSpaceType::Viewer,
            web_sys::XrReferenceSpaceType::Local => bevy_xr::interaction::XrReferenceSpaceType::Local,
            web_sys::XrReferenceSpaceType::LocalFloor => bevy_xr::interaction::XrReferenceSpaceType::Stage,
            _ => panic!("bevy_xr doesn't support XrReferenceSpaceType::{}", rf_space_type),
        }
    }
}

impl XrFrom<bevy_xr::interaction::XrReferenceSpaceType> for web_sys::XrReferenceSpaceType {
    fn xr_from(rf_space_type: bevy_xr::interaction::XrReferenceSpaceType) -> Self {
        match rf_space_type {
            bevy_xr::interaction::XrReferenceSpaceType::Viewer => web_sys::XrReferenceSpaceType::View,
            bevy_xr::interaction::XrReferenceSpaceType::Local => web_sys::XrReferenceSpaceType::Local,
            bevy_xr::interaction::XrReferenceSpaceType::Stage => web_sys::XrReferenceSpaceType::LocalFloor,
        }
    }
}

impl XrFrom<web_sys::XrHandedness> for bevy_xr::interaction::XrHandType {
    fn xr_from(handedness: web_sys::XrHandedness) -> Self {
        match handedness {
            web_sys::XrHandedness::Left => bevy_xr::interaction::XrHandType::Left,
            web_sys::XrHandedness::Right => bevy_xr::interaction::XrHandType::Right,
            _ => panic!("bevy_xr doesn't support XrHandType::{}", handedness)
        }
    }
}

impl bevy_xr::interaction::XrHandType for XrFrom<web_sys::XrHandedness>{
    fn xr_from(handedness: bevy_xr::interaction::XrHandType) -> Self {
        match handedness {
            bevy_xr::interaction::XrHandType::Left => web_sys::XrHandedness::Left,
            bevy_xr::interaction::XrHandType::Right => web_sys::XrHandedness::Right,
        }
    }
}

