use vulkano::device::{Features, Device, DeviceExtensions, Queue};
use vulkano::instance::{PhysicalDevice, InstanceExtensions, Instance};
use std::sync::Arc;

pub struct Engine {
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
}

impl Engine {
    pub fn new() -> Self {
        let instance = Instance::new(None, &InstanceExtensions::none(), None)
            .expect("failed to create instance");
        let physical = PhysicalDevice::enumerate(&instance).next().expect("no device available");
        let queue_family = physical.queue_families()
            .find(|&q| q.supports_compute())
            .expect("couldn't find a compute queue family");
        let device_ext = DeviceExtensions {
            khr_storage_buffer_storage_class: true,
            ..DeviceExtensions::none()
        };
        let (device, mut queues) = Device::new(physical, &Features::none(),
                                               &device_ext, [(queue_family, 0.5)].iter().cloned())
            .expect("failed to create device");
        let queue = queues.next().unwrap();

        Self {
            device,
            queue,
        }
    }
}
