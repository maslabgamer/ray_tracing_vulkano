use crate::{IMAGE_HEIGHT, IMAGE_WIDTH};

use vulkano::device::{Device, DeviceExtensions, Queue};
use vulkano::instance::{PhysicalDevice, Instance};
use std::sync::Arc;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::window::{WindowBuilder, Window};
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use vulkano::swapchain::{Surface, Swapchain, SurfaceTransform, FullscreenExclusive, ColorSpace, PresentMode, SwapchainCreationError};
use vulkano::image::{SwapchainImage, AttachmentImage};
use vulkano::framebuffer::{RenderPassAbstract, Framebuffer, FramebufferAbstract};
use vulkano::format::Format;
use vulkano::command_buffer::DynamicState;
use vulkano::pipeline::viewport::Viewport;
use vulkano::sync;
use vulkano::sync::GpuFuture;

pub struct Engine {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub instance: Arc<Instance>,
    pub surface: Arc<Surface<Window>>,
    pub(crate) swapchain: Arc<Swapchain<Window>>,
    pub(crate) recreate_swapchain: bool,
    pub(crate) images: Vec<Arc<SwapchainImage<Window>>>,
    pub(crate) dynamic_state: DynamicState,
    pub(crate) framebuffers: Vec<Arc<dyn FramebufferAbstract + Send + Sync>>,
    pub(crate) render_pass: Option<Arc<dyn RenderPassAbstract + Send + Sync>>,
    pub(crate) previous_frame_end: Option<Box<dyn GpuFuture>>,

    // Mouse stuff
    pub(crate) default_mouse_position: PhysicalPosition<i32>
}

impl Engine {
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        // Create Vulkano instance
        // Get required extensions to draw window
        let required_extensions = vulkano_win::required_extensions();
        let instance = Instance::new(None, &required_extensions, None)
            .expect("failed to create instance");

        // Create window
        let surface = match WindowBuilder::new()
            .with_title("Raytracer")
            .build_vk_surface(event_loop, instance.clone()) {
            Ok(surface) => surface,
            Err(e) => panic!("{}", e),
        };
        surface.window()
            // LogicalSize takes a type P that implements dpi::Pixel, which does not have
            // a usize implementation. Easy enough to cast to u32
            .set_inner_size(LogicalSize::new(IMAGE_WIDTH as u32, IMAGE_HEIGHT as u32));
        match surface.window().set_cursor_grab(true) {
            Ok(_) => println!("Got cursor lock on window."),
            Err(_) => panic!("Couldn't get cursor lock on window!"),
        }
        surface.window().set_cursor_visible(false);

        // Set default position for mouse
        let default_mouse_position = PhysicalPosition {
            x: IMAGE_WIDTH as i32 / 2,
            y: IMAGE_HEIGHT as i32 / 2,
        };

        // Grab the first available physical device
        let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");

        // Find an appropriate queue for this work
        let queue_family = physical.queue_families()
            .find(|&q|
                q.supports_compute() && surface.is_supported(q).unwrap_or(false)
            )
            .expect("couldn't find a compute queue family");

        // Enumerate required extensions we need to enable on the device
        let device_ext = DeviceExtensions {
            khr_storage_buffer_storage_class: true,
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        // Get a queue from our device and bind them appropriately
        let (device, mut queues) = Device::new(
            physical,
            physical.supported_features(),
            &device_ext,
            [(queue_family, 0.5)].iter().cloned(),
        ).expect("failed to create device");
        let queue = queues.next().unwrap();

        // Create the swapchain
        let dimensions: [u32; 2] = surface.window().inner_size().into();
        let (swapchain, images) = {
            // Query surface capabilities
            let caps = surface.capabilities(physical).unwrap();
            let usage = caps.supported_usage_flags;

            // Alpha mode indicates the alpha value of the final image will behave
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();

            // Choosing the internal format the images will have. Just take the first
            let format = caps.supported_formats[0].0;

            Swapchain::new(
                device.clone(),
                surface.clone(),
                caps.min_image_count,
                format,
                dimensions,
                1,
                usage,
                &queue,
                SurfaceTransform::Identity,
                alpha,
                PresentMode::Fifo,
                FullscreenExclusive::Default,
                true,
                ColorSpace::SrgbNonLinear,
            ).unwrap()
        };

        // Define our render pass
        let render_pass = Arc::new(
            vulkano::single_pass_renderpass!(
                device.clone(),
                attachments: {
                    color: {
                        load: Clear,
                        store: Store,
                        format: swapchain.format(),
                        samples: 1,
                    },
                    depth: {
                        load: Clear,
                        store: DontCare,
                        format: Format::D16Unorm,
                        samples: 1,
                    }
                },
                pass: {
                    color: [color],
                    depth_stencil: {depth}
                }
            )
                .unwrap(),
        );

        let mut dynamic_state = DynamicState {
            line_width: None,
            viewports: None,
            scissors: None,
            compare_mask: None,
            write_mask: None,
            reference: None,
        };

        // Create framebuffers
        let framebuffers = Engine::window_size_dependent_setup(
            device.clone(),
            &images,
            render_pass.clone(),
            &mut dynamic_state,
        );

        let previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        Self {
            device,
            queue,
            instance,
            surface,
            swapchain,
            recreate_swapchain: false, // Flag we set to recreate swapchain if need be
            images,
            dynamic_state,
            framebuffers,
            render_pass: Some(render_pass),
            previous_frame_end,
            default_mouse_position,
        }
    }

    // Called during initialization, then whenever window is resized
    pub(crate) fn window_size_dependent_setup(
        device: Arc<Device>,
        images: &[Arc<SwapchainImage<Window>>],
        render_pass: Arc<dyn RenderPassAbstract + Send + Sync>,
        dynamic_state: &mut DynamicState,
    ) -> Vec<Arc<dyn FramebufferAbstract + Send + Sync>> {
        let dimensions = images[0].dimensions();

        let viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [dimensions[0] as f32, dimensions[1] as f32],
            depth_range: 0.0..1.0,
        };
        dynamic_state.viewports = Some(vec![viewport]);

        let depth_buffer =
            AttachmentImage::transient(device.clone(), dimensions, Format::D16Unorm).unwrap();

        images
            .iter()
            .map(|image| {
                Arc::new(
                    Framebuffer::start(render_pass.clone())
                        .add(image.clone())
                        .unwrap()
                        .add(depth_buffer.clone())
                        .unwrap()
                        .build()
                        .unwrap(),
                ) as Arc<dyn FramebufferAbstract + Send + Sync>
            })
            .collect::<Vec<_>>()
    }

    pub fn recreate_swapchain(&mut self) {
        if self.recreate_swapchain {
            let dimensions: [u32; 2] = self.surface.window().inner_size().into();
            let (new_swapchain, new_images) =
                match self.swapchain.recreate_with_dimensions(dimensions) {
                    Ok(r) => r,
                    Err(SwapchainCreationError::UnsupportedDimensions) => return,
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

            self.swapchain = new_swapchain;
            let render_pass = self.render_pass.as_ref().unwrap();
            let new_framebuffers = Engine::window_size_dependent_setup(
                self.device.clone(),
                &new_images,
                render_pass.clone(),
                &mut self.dynamic_state,
            );
            self.framebuffers = new_framebuffers;
            self.images = new_images;
            self.recreate_swapchain = false;
        }
    }
}

