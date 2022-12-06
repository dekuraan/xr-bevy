use bevy_math::Vec3;
use std::sync::{Arc, Mutex};

use crate::conversion::{XrFrom, XrInto};
use wasm_bindgen::JsValue;

use js_sys::Object;

use web_sys::{
    DomPointReadOnly, XrBoundedReferenceSpace, XrFrame, XrHand, XrJointSpace, XrPose,
    XrReferenceSpace, XrReferenceSpaceType, XrView,
};

pub struct TrackingSource {
    space_type: XrReferenceSpaceType,
    space: Arc<Mutex<XrReferenceSpace>>,
    frame: XrFrame,
}

impl TrackingSource {
    pub fn new(space: XrReferenceSpace, space_type: XrReferenceSpaceType, frame: XrFrame) -> Self {
        Self {
            space: Arc::new(Mutex::new(space)),
            space_type,
            frame,
        }
    }
}

unsafe impl Send for TrackingSource {}
unsafe impl Sync for TrackingSource {}

impl bevy_xr::interaction::implementation::XrTrackingSourceBackend for TrackingSource {
    fn reference_space_type(&self) -> bevy_xr::XrReferenceSpaceType {
        self.space_type.xr_into()
    }

    fn set_reference_space_type(
        &self,
        _reference_space_type: bevy_xr::XrReferenceSpaceType,
    ) -> bool {
        // we can't set a diferent reference_space_type at runtime
        // because WebXr uses a Promise to do that and Bevy doesn't have async capabilities.
        // We can only set this before the App initialization at main function.
        //@TODO: fix when it's possible to use async code inside bevy
        false
    }

    fn bounds_geometry(&self) -> Option<Vec<Vec3>> {
        let space = XrBoundedReferenceSpace::from(
            <XrReferenceSpace as AsRef<JsValue>>::as_ref(&self.space.clone().lock().unwrap())
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
        let space = self.space.clone();
        let space = space.lock().unwrap();

        if let Some(viewer_pose) = self.frame.get_viewer_pose(&space) {
            return viewer_pose
                .views()
                .to_vec()
                .iter()
                .map(|js_value| XrView::from(js_value.clone()))
                .map(|view| bevy_xr::XrPose {
                    transform: view.transform().xr_into(),
                    linear_velocity: None,
                    angular_velocity: None,
                    emulated_position: viewer_pose.emulated_position(),
                })
                .collect();
        }
        vec![]
    }

    fn hands_pose(&self) -> [Option<bevy_xr::XrPose>; 2] {
        let left_input_src = self.frame.session().input_sources().get(0);
        let right_input_src = self.frame.session().input_sources().get(1);

        let base_space = self.space.clone();
        let base_space = base_space.lock().unwrap();

        return [
            left_input_src.map(|src| {
                self.frame
                    .get_pose(&src.grip_space().unwrap(), &base_space)
                    .unwrap()
                    .xr_into()
            }),
            right_input_src.map(|src| {
                self.frame
                    .get_pose(&src.grip_space().unwrap(), &base_space)
                    .unwrap()
                    .xr_into()
            }),
        ];
    }

    fn hands_skeleton_pose(&self) -> [Option<Vec<bevy_xr::XrJointPose>>; 2] {
        let left_input_src = self.frame.session().input_sources().get(0);
        let right_input_src = self.frame.session().input_sources().get(1);

        let base_space = self.space.clone();
        let base_space = base_space.lock().unwrap();

        return [
            left_input_src.map(|src| {
                Object::values(<XrHand as AsRef<js_sys::Object>>::as_ref(
                    &src.hand().unwrap(),
                ))
                .to_vec()
                .iter()
                .map(|js_value| XrJointSpace::from(js_value.clone()))
                .map(|joint_space| {
                    self.frame
                        .get_joint_pose(&joint_space, &base_space)
                        .unwrap()
                        .xr_into()
                })
                .collect()
            }),
            right_input_src.map(|src| {
                Object::values(<XrHand as AsRef<js_sys::Object>>::as_ref(
                    &src.hand().unwrap(),
                ))
                .to_vec()
                .iter()
                .map(|js_value| XrJointSpace::from(js_value.clone()))
                .map(|joint_space| {
                    self.frame
                        .get_joint_pose(&joint_space, &base_space)
                        .unwrap()
                        .xr_into()
                })
                .collect()
            }),
        ];
    }

    fn hands_target_ray(&self) -> [Option<bevy_xr::XrPose>; 2] {
        let left_input_src = self.frame.session().input_sources().get(0);
        let right_input_src = self.frame.session().input_sources().get(1);

        let base_space = self.space.clone();
        let base_space = base_space.lock().unwrap();

        return [
            left_input_src.map(|src| {
                self.frame
                    .get_pose(&src.target_ray_space(), &base_space)
                    .unwrap()
                    .xr_into()
            }),
            right_input_src.map(|src| {
                self.frame
                    .get_pose(&src.target_ray_space(), &base_space)
                    .unwrap()
                    .xr_into()
            }),
        ];
    }

    fn viewer_target_ray(&self) -> bevy_xr::XrPose {
        let base_space = self.space.clone();
        let base_space = base_space.lock().unwrap();

        let viewer_pose = self.frame.get_viewer_pose(&base_space).unwrap();

        XrFrom::<XrPose>::xr_from(XrPose::from(viewer_pose))
    }
}
