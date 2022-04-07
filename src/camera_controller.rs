use rapier::na::{self as nalgebra, vector};
use specs::{
    shred::DynamicSystemData,
    storage::{PairedStorage, SequentialRestriction},
    BitSet, Component, FlaggedStorage, System, VecStorage, World, Write, WriteStorage,
};
use std::time::Duration;
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceEvent, ElementState, KeyboardInput, MouseScrollDelta, VirtualKeyCode},
};

use crate::{
    actor::{DeviceInfo, Time},
    camera::*,
};

//const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.01;

#[derive(Debug, Component)]
#[storage(VecStorage)]
pub struct CameraController {
    amount_left: Real,
    amount_right: Real,
    amount_up: Real,
    amount_down: Real,
    // amount_forward: Real,
    // amount_backward: Real,
    rotate: Real,
    scroll: Real,
    speed: Real,
    base_speed: Real,
    fast_speed: Real,
    sensitivity: Real,
}

impl CameraController {
    pub fn new(speed: Real, sensitivity: Real) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            // amount_forward: 0.0,
            // amount_backward: 0.0,
            rotate: 0.0,
            scroll: 0.0,
            speed,
            base_speed: speed,
            fast_speed: speed * 2.0,
            sensitivity,
        }
    }

    pub fn process_event(&mut self, event: &DeviceEvent, info: &DeviceInfo) -> bool {
        match event {
            DeviceEvent::Key(KeyboardInput {
                virtual_keycode: Some(key),
                state,
                ..
            }) => {
                return self.process_keyboard(*key, *state);
            }
            DeviceEvent::MouseWheel { delta } => {
                self.process_scroll(delta);
                false
            }
            DeviceEvent::MouseMotion { delta } => match info.mouse_map.get(&1) {
                Some(state) => {
                    let (_, state) = state.pair();
                    if *state == ElementState::Pressed {
                        self.process_mouse(delta.0, delta.1);
                    }
                    false
                }
                None => false,
            },
            _ => false,
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };

        match key {
            VirtualKeyCode::A | VirtualKeyCode::Left => {
                self.amount_left = amount;
                true
            }
            VirtualKeyCode::D | VirtualKeyCode::Right => {
                self.amount_right = amount;
                true
            }
            VirtualKeyCode::W | VirtualKeyCode::Up => {
                //self.amount_forward = amount;
                self.amount_up = amount;
                true
            }
            VirtualKeyCode::S | VirtualKeyCode::Down => {
                //self.amount_backward = amount;
                self.amount_down = amount;
                true
            }
            /*VirtualKeyCode::LControl => {
                self.amount_down = amount;
                true
            }
            VirtualKeyCode::Space => {
                self.amount_up = amount;
                true
            }*/
            VirtualKeyCode::LShift => {
                self.speed = match state {
                    ElementState::Pressed => self.fast_speed,
                    _ => self.base_speed,
                };
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if mouse_dx != 0.0 {
            //self.rotate_horizontal = mouse_dx as f32;
        }

        if mouse_dy != 0.0 {
            //self.rotate_vertical = -mouse_dy as f32;
        }
    }

    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        let scroll = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 1.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };

        if scroll != 0.0 {
            self.scroll = scroll;
        }
    }

    pub fn update_camera(
        &mut self,
        position: &mut PairedStorage<
            Position,
            &mut FlaggedStorage<Position>,
            &BitSet,
            SequentialRestriction,
        >,
        rotation: &mut PairedStorage<
            Rotation,
            &mut FlaggedStorage<Rotation>,
            &BitSet,
            SequentialRestriction,
        >,
        dt: Duration,
    ) {
        let dt = dt.as_secs_f32();

        /*let (roll, mut pitch, mut yaw) = rotation.get_unchecked().0.euler_angles(); // try to simplify please

        let delta_x = self.amount_right - self.amount_left;
        let delta_y = self.amount_up - self.amount_down;
        let delta_z = self.amount_forward - self.amount_backward;

        let (roll_sin, roll_cos) =
            if delta_z != 0.0 || self.rotate_horizontal != 0.0 || self.rotate_vertical != 0.0 {
                roll.sin_cos()
            } else {
                (0.0, 0.0)
            };

        let (pitch_sin, pitch_cos) = if delta_z != 0.0 {
            pitch.sin_cos()
        } else {
            (0.0, 0.0)
        };

        let (yaw_sin, yaw_cos) = if delta_x != 0.0 || delta_z != 0.0 {
            yaw.sin_cos()
        } else {
            (0.0, 0.0)
        };

        if delta_x != 0.0 || delta_y != 0.0 || delta_z != 0.0 {
            let mut delta_position: Vector = Vector::zeros();

            if delta_x != 0.0 {
                delta_position += delta_x * vector![-yaw_sin, 0.0, yaw_cos];
            }

            if delta_y != 0.0 {
                delta_position += delta_y * vector![0.0, 1.0, 0.0];
            }

            if delta_z != 0.0 {
                delta_position +=
                    delta_z * vector![pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin];
            }

            delta_position *= self.speed * dt;

            if delta_position != Vector::zeros() {
                position.get_mut_unchecked().0 += delta_position;
            }
        }*/

        let mut angle = rotation.get_unchecked().0.angle(); // try to simplify please

        let delta_x = self.amount_left - self.amount_right;
        let delta_y = self.amount_up - self.amount_down;

        let (sin, cos) = if self.rotate != 0.0 {
            angle.sin_cos()
        } else {
            (0.0, 1.0)
        };

        if delta_x != 0.0 || delta_y != 0.0 {
            let mut delta_position: Vector = Vector::zeros();

            if delta_x != 0.0 {
                delta_position += delta_x * vector![cos, sin];
            }

            if delta_y != 0.0 {
                delta_position += delta_y * vector![sin, cos];
            }

            delta_position *= self.speed * dt;

            if delta_position != Vector::zeros() {
                position.get_mut_unchecked().0 += delta_position;
            }
        }

        // Move in/out (aka. "zoom")
        // Note: this isn't an actual zoom. The camera's position
        // changes when zooming. I've added this to make it easier
        // to get closer to an object you want to focus on.
        //let scrollward: Vector3<f32> =
        //    Vector3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        //*position += scrollward * self.scroll * self.speed * self.sensitivity * 1.0;

        // Move up/down. Since we don't use roll, we can just
        // modify the y coordinate directly.

        // Rotate
        if self.rotate != 0.0 {
            let angle_delta = self.rotate * self.sensitivity * 0.01;

            // If process_mouse isn't called every frame, these values
            // will not get set to zero, and the camera will rotate
            // when moving in a non cardinal direction.

            if angle_delta != 0.0 {
                angle += angle_delta;
                rotation.get_mut_unchecked().0 = Rotator::new(angle);
            }
        }

        self.rotate = 0.0;
        self.scroll = 0.0;
    }
}

#[derive(Default)]
pub struct CameraControllerSys;

impl<'a> System<'a> for CameraControllerSys {
    type SystemData = (
        WriteStorage<'a, CameraController>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Rotation>,
        Write<'a, Time>,
    );

    fn run(
        &mut self,
        (mut camera_controller, mut position, mut rotation, mut time): Self::SystemData,
    ) {
        use specs::Join;

        for (camera_controller, mut position, mut rotation) in (
            &mut camera_controller,
            &mut position.restrict_mut(),
            &mut rotation.restrict_mut(),
        )
            .join()
        {
            if camera_controller.scroll != 0.0 {
                time.on = camera_controller.scroll > 0.0;
            }
            camera_controller.update_camera(&mut position, &mut rotation, time.delta);
        }
    }

    fn setup(&mut self, world: &mut World) {
        Self::SystemData::setup(&self.accessor(), world);
    }
}
