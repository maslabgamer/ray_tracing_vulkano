mod sphere;
mod camera;
mod engine;

use crate::sphere::Sphere;
use std::sync::Arc;
use vulkano::image::{StorageImage, Dimensions};
use vulkano::format::Format;
use vulkano::buffer::{BufferUsage, CpuBufferPool, CpuAccessibleBuffer};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBuffer};
use vulkano::sync::GpuFuture;
use vulkano::pipeline::ComputePipeline;
use image::{ImageBuffer, Rgba};
use vulkano::descriptor::descriptor_set::PersistentDescriptorSet;
use vulkano::descriptor::PipelineLayoutAbstract;
use crate::camera::Camera;
use crate::engine::Engine;

const IMAGE_WIDTH: u32 = 1920;
const IMAGE_HEIGHT: u32 = 1080;

fn main() {
    let engine = Engine::new();

    let scene: [Sphere; 3] = [
        Sphere::new(0.0, -1.0, 3.0, 1, &[255, 0, 0, 0]),
        Sphere::new(2.0, 0.0, 4.0, 1, &[0, 0, 255, 0]),
        Sphere::new(-2.0, 0.0, 4.0, 1, &[0, 255, 0, 0]),
    ];

    let shader = cs::Shader::load(engine.device.clone()).expect("failed to create shader module");

    let compute_pipeline = Arc::new(
        ComputePipeline::new(engine.device.clone(), &shader.main_entry_point(), &(), None)
            .expect("failed to create compute pipeline")
    );

    // Initialize camera uniform buffer
    let camera = Camera::from_origin();

    let camera_buffer = CpuBufferPool::<cs::ty::Camera>::new(engine.device.clone(), BufferUsage::all());

    let camera_subbuffer = Arc::new(camera_buffer.next(camera.to_uniform()).unwrap());

    // Initialize spheres uniform buffer
    let spheres_buffer = CpuBufferPool::<cs::ty::Spheres>::new(engine.device.clone(), BufferUsage::all());

    let spheres_buffer_subbuffer = {
        let spheres: [cs::ty::Sphere; 3] = [
            scene[0].to_uniform(),
            scene[1].to_uniform(),
            scene[2].to_uniform(),
        ];

        let uniform_data = cs::ty::Spheres {
            instances: spheres.into(),
        };

        Arc::new(spheres_buffer.next(uniform_data).unwrap())
    };

    // Create an image
    let image = StorageImage::new(
        engine.device.clone(),
        Dimensions::Dim2d { width: IMAGE_WIDTH, height: IMAGE_HEIGHT },
        Format::R8G8B8A8Unorm,
        Some(engine.queue.family()),
    ).unwrap();

    let layout = compute_pipeline.layout().descriptor_set_layout(0).unwrap();
    let set = Arc::new(
        PersistentDescriptorSet::start(layout.clone())
            .add_image(image.clone()).unwrap() // Image we write to
            .add_buffer(camera_subbuffer.clone()).unwrap() // Camera uniform
            .add_buffer(spheres_buffer_subbuffer.clone()).unwrap() // Spheres uniform
            .build().unwrap()
    );

    let buf = CpuAccessibleBuffer::from_iter(
        engine.device.clone(),
        BufferUsage::all(),
        false,
        (0..IMAGE_HEIGHT * IMAGE_WIDTH * 4).map(|_| 0u8),
    ).expect("failed to create buffer");

    // Create command buffer with a draw command dispatch followed by a copy image to buffer command
    let mut builder = AutoCommandBufferBuilder::new(
        engine.device.clone(),
        engine.queue.family(),
    ).unwrap();
    builder.dispatch(
        [IMAGE_WIDTH / 8, IMAGE_HEIGHT / 8, 1],
        compute_pipeline.clone(),
        set.clone(), (),
    ).unwrap()
        .copy_image_to_buffer(image.clone(), buf.clone()).unwrap();
    let command_buffer = builder.build().unwrap();

    // Execute draw command and save resulting image
    let finished = command_buffer.execute(engine.queue.clone()).unwrap();
    finished.then_signal_fence_and_flush().unwrap().wait(None).unwrap();

    let buffer_content = buf.read().unwrap();
    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
        IMAGE_WIDTH,
        IMAGE_HEIGHT,
        &buffer_content[..]
    ).unwrap();
    image.save("image.png").unwrap();
}

mod cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "src/shaders/shader.comp"
    }
}
