mod sphere;
mod camera;
mod engine;
mod light;
mod object_traits;

use crate::sphere::Sphere;
use crate::object_traits::Uniform;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuBufferPool};
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::sync::{GpuFuture, FlushError};
use vulkano::pipeline::ComputePipeline;
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::PipelineLayoutAbstract;
use crate::camera::Camera;
use crate::engine::Engine;
use crate::light::{Light, LightType};
use cgmath::{Vector3, InnerSpace, Rad, Angle};
use std::time::Instant;
use device_query::{Keycode, DeviceState, DeviceQuery};
use std::f32::consts::FRAC_PI_2;
use winit::event_loop::{EventLoop, ControlFlow};
use winit::event::{Event, WindowEvent};
use vulkano::swapchain::AcquireError;
use vulkano::swapchain;
use vulkano::sync;

const IMAGE_WIDTH: usize = 1080;
const IMAGE_HEIGHT: usize = 1080;

fn main() {
    // Create event loop for window
    let event_loop = EventLoop::new();
    let mut engine = Engine::new(&event_loop);

    // Set up Spheres
    let scene: [Sphere; 4] = [
        Sphere::new(
            0.0, -1.0, 3.0,
            1,
            &[1.0, 0.0, 0.0, 0.0],
            500,
            0.2,
        ),
        Sphere::new(2.0, 0.0, 4.0,
                    1,
                    &[0.0, 0.0, 1.0, 0.0],
                    500,
                    0.3,
        ),
        Sphere::new(-2.0, 0.0, 4.0,
                    1,
                    &[0.0, 1.0, 0.0, 0.0],
                    10,
                    0.4,
        ),
        Sphere::new(0.0, -5001.0, 4.0,
                    5000,
                    &[1.0, 1.0, 0.0, 0.0],
                    1000,
                    0.5,
        ),
    ];

    // Set up Lights
    let lights: [Light; 3] = [
        Light::new(LightType::Ambient, 0.2, None),
        Light::new(LightType::Point, 0.6, Some(Vector3::new(2.0, 1.0, 0.0))),
        Light::new(LightType::Directional, 0.2, Some(Vector3::new(1.0, 4.0, 4.0))),
    ];

    let shader = cs::Shader::load(engine.device.clone()).expect("failed to create shader module");

    let compute_pipeline = Arc::new(
        ComputePipeline::new(engine.device.clone(), &shader.main_entry_point(), &(), None)
            .expect("failed to create compute pipeline")
    );

    // Initialize camera uniform buffer
    let mut camera = Camera::from_origin();
    let camera_buffer = CpuBufferPool::<cs::ty::Camera>::new(engine.device.clone(), BufferUsage::all());
    // Initialize spheres uniform buffer
    let spheres_buffer = CpuBufferPool::<cs::ty::Spheres>::new(engine.device.clone(), BufferUsage::all());

    let spheres_buffer_subbuffer = {
        let spheres: [cs::ty::Sphere; 4] = [
            scene[0].to_uniform(),
            scene[1].to_uniform(),
            scene[2].to_uniform(),
            scene[3].to_uniform(),
        ];

        let uniform_data = cs::ty::Spheres {
            instances: spheres.into(),
        };

        Arc::new(spheres_buffer.next(uniform_data).unwrap())
    };

    // Initialize lights uniform buffer
    let lights_buffer = CpuBufferPool::<cs::ty::Lights>::new(engine.device.clone(), BufferUsage::all());

    let lights_buffer_subbuffer = {
        let lights: [cs::ty::Light; 3] = [
            lights[0].to_uniform(),
            lights[1].to_uniform(),
            lights[2].to_uniform(),
        ];

        let uniform_data = cs::ty::Lights {
            instances: lights.into(),
        };

        Arc::new(lights_buffer.next(uniform_data).unwrap())
    };

    // Set up input handlers
    let device_state = DeviceState::new();
    let mut window_is_focused = true; // Assume focused at startup

    // Set up delta_time timer
    let mut delta_timer = Instant::now();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        // Update delta time
        let now = Instant::now();
        let dt = (now - delta_timer).as_secs_f32();
        delta_timer = now;

        // Handle keyboard input
        let keys: Vec<Keycode> = device_state.get_keys();
        if !keys.is_empty() {
            let (yaw_sin, yaw_cos) = (-camera.yaw).sin_cos();
            let forward = Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
            let right = Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();

            for key in keys {
                match key {
                    Keycode::Escape => *control_flow = ControlFlow::Exit,
                    Keycode::W => camera.position += forward * camera.speed * dt,
                    Keycode::S => camera.position -= forward * camera.speed * dt,
                    Keycode::A => camera.position += right * camera.speed * dt,
                    Keycode::D => camera.position -= right * camera.speed * dt,
                    Keycode::Space => camera.position.y += camera.speed * dt,
                    Keycode::LShift => camera.position.y -= camera.speed * dt,
                    _ => {}
                }
            }
        }

        // Process window events
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => engine.recreate_swapchain = true,
            Event::WindowEvent {
                event: WindowEvent::Focused(in_focus),
                ..
            } => {
                window_is_focused = in_focus;
                engine.surface.window().set_cursor_visible(!window_is_focused);
            }
            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                if window_is_focused {
                    // Handle mouse input
                    let x_difference = position.x - engine.default_mouse_position.x as f64;
                    let y_difference = position.y - engine.default_mouse_position.y as f64;

                    camera.yaw += Rad(x_difference as f32) * camera.sensitivity * dt;
                    camera.pitch += Rad(-y_difference as f32) * camera.sensitivity * dt;

                    if camera.pitch < -Rad(FRAC_PI_2) {
                        camera.pitch = -Rad(FRAC_PI_2);
                    } else if camera.pitch > Rad(FRAC_PI_2) {
                        camera.pitch = Rad(FRAC_PI_2);
                    }

                    if let Err(_) = engine.surface.window().set_cursor_position(engine.default_mouse_position) {
                        panic!("Could not set cursor position!");
                    }
                }
            }
            Event::RedrawEventsCleared => {
                // Clean up unused resources
                engine.previous_frame_end.as_mut().unwrap().cleanup_finished();

                // Whenever window resizes we need to recreate everything dependent on the window size.
                engine.recreate_swapchain();

                // Have to acquire an images from the swapchain before we can draw it
                let (image_num, suboptimal, acquire_future) =
                    match swapchain::acquire_next_image(engine.swapchain.clone(), None) {
                        Ok(r) => r,
                        Err(AcquireError::OutOfDate) => {
                            engine.recreate_swapchain = true;
                            return;
                        }
                        Err(e) => panic!("Failed to acquire next image: {:?}", e),
                    };

                if suboptimal {
                    engine.recreate_swapchain = true;
                }

                // Update view
                let camera_subbuffer = Arc::new(camera_buffer.next(camera.to_uniform()).unwrap());

                // Create command buffer with a draw command dispatch followed by a copy image to buffer command
                let command_buffer = {
                    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();
                    let set = Arc::new(
                        PersistentDescriptorSet::start(layout.clone())
                            .add_image(engine.images[image_num].clone()).unwrap() // Image we write to
                            .add_buffer(camera_subbuffer.clone()).unwrap() // Camera uniform
                            .add_buffer(spheres_buffer_subbuffer.clone()).unwrap() // Spheres uniform
                            .add_buffer(lights_buffer_subbuffer.clone()).unwrap() // Lights uniform
                            .build().unwrap()
                    );

                    let mut command_buffer = AutoCommandBufferBuilder::new(engine.device.clone(), engine.queue.family())
                        .unwrap();
                    command_buffer.dispatch(
                        [IMAGE_WIDTH as u32 / 8, IMAGE_HEIGHT as u32 / 8, 1],
                        compute_pipeline.clone(),
                        set.clone(),
                        (),
                    )
                        .unwrap();

                    command_buffer.build().unwrap()
                };

                // Execute draw command and save resulting image
                let future = engine.previous_frame_end
                    .take()
                    .unwrap()
                    .join(acquire_future)
                    .then_execute(engine.queue.clone(), command_buffer)
                    .unwrap()
                    .then_swapchain_present(engine.queue.clone(), engine.swapchain.clone(), image_num)
                    .then_signal_fence_and_flush();

                match future {
                    Ok(future) => engine.previous_frame_end = Some(Box::new(future) as Box<_>),
                    Err(FlushError::OutOfDate) => {
                        engine.recreate_swapchain = true;
                        engine.previous_frame_end = Some(Box::new(sync::now(engine.device.clone())) as Box<_>)
                    }
                    Err(e) => {
                        println!("Failed to flush future: {:?}", e);
                        engine.previous_frame_end = Some(Box::new(sync::now(engine.device.clone())) as Box<_>);
                    }
                }
            }
            _ => {}
        }
    });
}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "src/shaders/shader.comp"
    }
}
