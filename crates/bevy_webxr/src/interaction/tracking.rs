use web_sys;

use crate::interaction::conversion::*;
use bevy_xr;
use bevy::ResMut;


pub struct WebXrInteractionContext {
    space_type: web_sys::XrReferenceSpaceType,
    space: web_sys::XrReferenceSpace,
    frame: web_sys::XrFrame,
}


impl WebXrInteractionContext {

    pub fn new(space_type: web_sys::XrReferenceSpaceType, space: web_sys::XrReferenceSpace) -> Self {
        Self {
            space_type,
            space,
        }
    }
}

impl bevy_xr::interaction::implementation::XrTrackingSourceBackend for WebXrInteractionContext {
    fn reference_space_type(&self) -> bevy_xr::XrReferenceSpaceType {
        self.space_type.xr_into()
    }

    fn set_reference_space_type(&self, reference_space_type: XrReferenceSpaceType) -> bool {
        // we can't set a diferent reference_space_type at runtime
        // because WebXr uses a Promise to do that and Bevy doesn't have async capabilities.
        // We can only set this before the App initialization at main function.
        false         
    }

    fn bounds_geometry(&self) -> Option<Vec<Vec3>> {
        let space = web_sys::XrBoundedReferenceSpace::from(self.space).ok()?;
        Some(
            space.bounds_geometry()
            .map(|js_value| web_sys::DomPointReadOnly::from(js_value))
            .map(|point| Vec3::new(point.x(), point.y(), point.z()))
            .collect()
        )
    }

    fn views_poses(&self) -> Vec<bevy_xr::XrPose> {
        self.space.  
    }

    fn hands_pose(&self) -> [Option<bevy_xr::XrPose>; 2]{}

    fn hands_skeleton_pose(&self) -> [Option<Vec<bevy_xr::XrJointPose>>; 2]{}

    fn hands_target_ray(&self) -> [Option<bevy_xr::XrPose>; 2]{}

    fn viewer_target_ray(&self) -> bevy_xr::XrPose{}
}

pub(crate) fn update_interaction_context(webxr_ctx: ResMut<WebXrContext>) {
    webxr_ctx.request_animation_frame(|_, frame| webxr_ctx.interaction_ctx.frame = Some(Frame))
}
