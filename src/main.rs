mod actor;
mod camera;
mod camera_controller;
mod camera_uniform;
mod fs;
mod instance_uniform;
//mod light;
mod model;
mod state;
mod texture;
mod type_def;
mod buffer_update;
mod sprite_selector;
mod collider;
mod rigid_body;

extern crate rapier2d as rapier;

use rapier::{na::Matrix4};
use specs::{WorldExt};
use state::*;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
pub use crate::type_def::*;

pub const SHADOW_SIZE: u32 = 1024;

pub const DEG_TO_RAD: Real = std::f64::consts::PI as Real / 180.0;
// converts angles from degrees to radians
pub fn deg(deg: Real) -> Real {
    deg * DEG_TO_RAD
}

pub const RAD_TO_DEG: Real = 180.0 / std::f64::consts::PI as Real;
// converts angles from radians to degrees
pub fn to_deg(rad: Real) -> Real {
    rad * RAD_TO_DEG
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut last_render_time = std::time::Instant::now();
    let mut state = pollster::block_on(State::new(&window));

    println!(
        "time: {}ms",
        last_render_time.elapsed().as_secs_f64() * 1000.0
    );

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                // print!("\r{}\tFPS", (1.0 / dt.as_secs_f64()) as usize);
                state.update(dt);
                state.reset_input();
                
                match state.world.write_resource::<actor::ControlFlow>().0.take() {
                    Some(control_flow_read) => {
                        *control_flow = control_flow_read; 
                    },
                    None => {}
                }
            }
            Event::MainEventsCleared => {
                // RedrawRequested will only trigger once, unless we manually
                // request it.
                window.request_redraw();
            }
            Event::DeviceEvent {
                ref event,
                .. // We're not using device_id currently
            } => {
                state.input(event);
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });
}
