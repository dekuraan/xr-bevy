use bevy_math::Vec3;
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsValue;

use crate::XrInto;

use web_sys::{
    DomPointReadOnly, XrBoundedReferenceSpace, XrFrame, XrHandedness, XrInputSourceArray, XrPose,
    XrReferenceSpace, XrReferenceSpaceType, XrSession,
};

pub struct WebXrInteractionContext {
    space_type: XrReferenceSpaceType,
    space: Arc<Mutex<XrReferenceSpace>>,
    sources: Arc<Mutex<XrInputSourceArray>>,
    pub frame: Option<XrFrame>,
}

impl WebXrInteractionContext {
    pub fn new(
        session: &XrSession,
        space_type: XrReferenceSpaceType,
        space: XrReferenceSpace,
    ) -> Self {
        Self {
            space_type,
            space: Arc::new(Mutex::new(space)),
            sources: Arc::new(Mutex::new(session.input_sources())),
            frame: None,
        }
    }
}

unsafe impl Send for WebXrInteractionContext {}
unsafe impl Sync for WebXrInteractionContext {}

pub struct TrackingSource {
    context: WebXrInteractionContext,
}

impl bevy_xr::interaction::implementation::XrTrackingSourceBackend for TrackingSource {
    fn reference_space_type(&self) -> bevy_xr::XrReferenceSpaceType {
        self.context.space_type.xr_into()
    }

    fn set_reference_space_type(
        &self,
        _reference_space_type: bevy_xr::XrReferenceSpaceType,
    ) -> bool {
        // we can't set a diferent reference_space_type at runtime
        // because WebXr uses a Promise to do that and Bevy doesn't have async capabilities.
        // We can only set this before the App initialization at main function.
        false
    }

    fn bounds_geometry(&self) -> Option<Vec<Vec3>> {
        let space = XrBoundedReferenceSpace::from(
            <XrReferenceSpace as AsRef<JsValue>>::as_ref(
                &self.context.space.clone().lock().unwrap(),
            )
            .clone(),
        );
        Some(
            space
                .bounds_geometry()
                .to_vec()
                .iter()
                .map(|js_value| DomPointReadOnly::from(js_value.clone()))
                .map(|point| Vec3::new(point.x() as f32, point.y() as f32, point.z() as f32))
                .collect(),
        )
    }

    fn views_poses(&self) -> Vec<bevy_xr::XrPose> {
        todo!()
    }

    fn hands_pose(&self) -> [Option<bevy_xr::XrPose>; 2] {
        if let Some(frame) = &self.context.frame {
            let left_hand_input_src = {self.context.sources.clone()}.lock().unwrap().get(0);
            let right_hand_input_src = {self.context.sources.clone()}.lock().unwrap().get(1);

            let base_space = self.context.space.clone();
            let base_space = base_space.lock().unwrap();

            return [
                left_hand_input_src.map(|src| {
                    frame.get_pose(&src.grip_space().unwrap(), &base_space).unwrap().xr_into()
                }),
                right_hand_input_src.map(|src| {
                    frame.get_pose(&src.grip_space().unwrap(), &base_space).unwrap().xr_into()
                }),
            ]
        }
        [None, None]
    }

    fn hands_skeleton_pose(&self) -> [Option<Vec<bevy_xr::XrJointPose>>; 2] {
        todo!()
    }

    fn hands_target_ray(&self) -> [Option<bevy_xr::XrPose>; 2] {
        todo!()
    }

    fn viewer_target_ray(&self) -> bevy_xr::XrPose {
        todo!()
    }
}
