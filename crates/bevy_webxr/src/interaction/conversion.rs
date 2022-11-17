use crate::{XrFrom, XrInto};
use bevy_math::{Quat, Vec3};

use web_sys::{
    DomPointInit, XrHandedness, XrJointPose, XrPose, XrReferenceSpaceType, XrRigidTransform,
};

impl XrFrom<XrRigidTransform> for bevy_xr::interaction::XrRigidTransform {
    fn xr_from(rigid_transform: XrRigidTransform) -> Self {
        let position = rigid_transform.position();
        let orientation = rigid_transform.orientation();

        bevy_xr::interaction::XrRigidTransform {
            position: Vec3::new(
                position.x() as f32,
                position.y() as f32,
                position.z() as f32,
            ),
            orientation: Quat::from_xyzw(
                orientation.x() as f32,
                orientation.y() as f32,
                orientation.z() as f32,
                orientation.w() as f32,
            ),
        }
    }
}

impl XrFrom<bevy_xr::interaction::XrRigidTransform> for XrRigidTransform {
    fn xr_from(rigid_transform: bevy_xr::interaction::XrRigidTransform) -> Self {
        let mut position = DomPointInit::new();
        position.x(rigid_transform.position.x.into());
        position.y(rigid_transform.position.y.into());
        position.z(rigid_transform.position.z.into());

        let quat_array = rigid_transform.orientation.to_array();
        let mut orientation = DomPointInit::new();
        orientation.x(quat_array[0].into());
        orientation.y(quat_array[1].into());
        orientation.z(quat_array[2].into());
        orientation.w(quat_array[3].into());

        XrRigidTransform::new_with_position_and_orientation(&position, &orientation)
            .expect("Failed to cast from bevy_xr::XrRigidTransform to web_sys::XrRigidTransform")
    }
}

impl XrFrom<XrPose> for bevy_xr::interaction::XrPose {
    fn xr_from(pose: XrPose) -> Self {
        bevy_xr::interaction::XrPose {
            transform: pose.transform().xr_into(),
            linear_velocity: pose
                .linear_velocity()
                .map(|point| Vec3::new(point.x() as f32, point.y() as f32, point.z() as f32)),
            angular_velocity: pose
                .angular_velocity()
                .map(|point| Vec3::new(point.x() as f32, point.y() as f32, point.z() as f32)),
            emulated_position: pose.emulated_position(),
        }
    }
}

impl XrFrom<XrJointPose> for bevy_xr::interaction::XrJointPose {
    fn xr_from(pose: XrJointPose) -> Self {
        bevy_xr::interaction::XrJointPose {
            pose: bevy_xr::interaction::XrPose {
                transform: pose.transform().xr_into(),
                linear_velocity: pose
                    .linear_velocity()
                    .map(|point| Vec3::new(point.x() as f32, point.y() as f32, point.z() as f32)),
                angular_velocity: pose
                    .angular_velocity()
                    .map(|point| Vec3::new(point.x() as f32, point.y() as f32, point.z() as f32)),
                emulated_position: pose.emulated_position(),
            },
            radius: pose.radius(),
        }
    }
}

impl XrFrom<XrReferenceSpaceType> for bevy_xr::interaction::XrReferenceSpaceType {
    fn xr_from(rf_space_type: XrReferenceSpaceType) -> Self {
        match rf_space_type {
            XrReferenceSpaceType::Viewer => bevy_xr::interaction::XrReferenceSpaceType::Viewer,
            XrReferenceSpaceType::Local => bevy_xr::interaction::XrReferenceSpaceType::Local,
            XrReferenceSpaceType::LocalFloor => bevy_xr::interaction::XrReferenceSpaceType::Stage,
            _ => panic!(
                "bevy_xr doesn't support XrReferenceSpaceType::{:?}",
                rf_space_type
            ),
        }
    }
}

impl XrFrom<bevy_xr::interaction::XrReferenceSpaceType> for XrReferenceSpaceType {
    fn xr_from(rf_space_type: bevy_xr::interaction::XrReferenceSpaceType) -> Self {
        match rf_space_type {
            bevy_xr::interaction::XrReferenceSpaceType::Viewer => XrReferenceSpaceType::Viewer,
            bevy_xr::interaction::XrReferenceSpaceType::Local => XrReferenceSpaceType::Local,
            bevy_xr::interaction::XrReferenceSpaceType::Stage => XrReferenceSpaceType::LocalFloor,
        }
    }
}

impl XrFrom<XrHandedness> for bevy_xr::interaction::XrHandType {
    fn xr_from(handedness: XrHandedness) -> Self {
        match handedness {
            XrHandedness::Left => bevy_xr::interaction::XrHandType::Left,
            XrHandedness::Right => bevy_xr::interaction::XrHandType::Right,
            _ => panic!("bevy_xr doesn't support XrHandType::{:?}", handedness),
        }
    }
}

impl XrFrom<bevy_xr::interaction::XrHandType> for XrHandedness {
    fn xr_from(handedness: bevy_xr::interaction::XrHandType) -> Self {
        match handedness {
            bevy_xr::interaction::XrHandType::Left => XrHandedness::Left,
            bevy_xr::interaction::XrHandType::Right => XrHandedness::Right,
        }
    }
}
