use bevy_math::{Quat, Vec3};
use web_sys::DomPointInit;

// WebXR <-> Bevy XR Conversion
pub(crate) trait XrFrom<T> {
    fn xr_from(_: T) -> Self;
}

pub(crate) trait XrInto<T> {
    fn xr_into(self) -> T;
}

impl<T, U> XrInto<U> for T
where
    U: XrFrom<T>,
{
    fn xr_into(self) -> U {
        U::xr_from(self)
    }
}

// XR Conversion Impls

impl XrFrom<web_sys::XrSessionMode> for bevy_xr::XrSessionMode {
    fn xr_from(mode: web_sys::XrSessionMode) -> Self {
        match mode {
            web_sys::XrSessionMode::Inline => bevy_xr::XrSessionMode::InlineVR,
            web_sys::XrSessionMode::ImmersiveVr => bevy_xr::XrSessionMode::ImmersiveVR,
            web_sys::XrSessionMode::ImmersiveAr => bevy_xr::XrSessionMode::ImmersiveAR,
            _ => panic!("Invalid XrSessionMode"),
        }
    }
}

impl XrFrom<bevy_xr::XrSessionMode> for web_sys::XrSessionMode {
    fn xr_from(web_xr: bevy_xr::XrSessionMode) -> Self {
        match web_xr {
            bevy_xr::XrSessionMode::ImmersiveVR => web_sys::XrSessionMode::ImmersiveVr,
            bevy_xr::XrSessionMode::ImmersiveAR => web_sys::XrSessionMode::ImmersiveAr,
            bevy_xr::XrSessionMode::InlineVR => web_sys::XrSessionMode::Inline,
            //TODO: remove bevy_xr::XrSessionMode::InlineAr?
            bevy_xr::XrSessionMode::InlineAR => web_sys::XrSessionMode::Inline,
        }
    }
}

impl XrFrom<web_sys::XrReferenceSpaceType> for bevy_xr::interaction::XrReferenceSpaceType {
    fn xr_from(rf_space_type: web_sys::XrReferenceSpaceType) -> Self {
        match rf_space_type {
            web_sys::XrReferenceSpaceType::Viewer => {
                bevy_xr::interaction::XrReferenceSpaceType::Viewer
            }
            web_sys::XrReferenceSpaceType::Local => {
                bevy_xr::interaction::XrReferenceSpaceType::Local
            }
            web_sys::XrReferenceSpaceType::LocalFloor => {
                bevy_xr::interaction::XrReferenceSpaceType::Stage
            }
            _ => panic!(
                "bevy_xr doesn't support XrReferenceSpaceType::{:?}",
                rf_space_type
            ),
        }
    }
}

impl XrFrom<bevy_xr::interaction::XrReferenceSpaceType> for web_sys::XrReferenceSpaceType {
    fn xr_from(rf_space_type: bevy_xr::interaction::XrReferenceSpaceType) -> Self {
        match rf_space_type {
            bevy_xr::interaction::XrReferenceSpaceType::Viewer => {
                web_sys::XrReferenceSpaceType::Viewer
            }
            bevy_xr::interaction::XrReferenceSpaceType::Local => {
                web_sys::XrReferenceSpaceType::Local
            }
            bevy_xr::interaction::XrReferenceSpaceType::Stage => {
                web_sys::XrReferenceSpaceType::LocalFloor
            }
        }
    }
}

impl XrFrom<web_sys::XrRigidTransform> for bevy_xr::interaction::XrRigidTransform {
    fn xr_from(rigid_transform: web_sys::XrRigidTransform) -> Self {
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

impl XrFrom<bevy_xr::interaction::XrRigidTransform> for web_sys::XrRigidTransform {
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

        web_sys::XrRigidTransform::new_with_position_and_orientation(&position, &orientation)
            .expect("Failed to cast from bevy_xr::XrRigidTransform to web_sys::XrRigidTransform")
    }
}

impl XrFrom<web_sys::XrPose> for bevy_xr::interaction::XrPose {
    fn xr_from(pose: web_sys::XrPose) -> Self {
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

impl XrFrom<web_sys::XrJointPose> for bevy_xr::interaction::XrJointPose {
    fn xr_from(pose: web_sys::XrJointPose) -> Self {
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
