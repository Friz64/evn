pub mod shaders;

use winit::Window;

// renderer is fully accessed over the ecs
pub struct Renderer {
    pub window: Window,
}

impl Renderer {
    pub fn new(window: Window) -> Self {
        Renderer { window }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // destory all vulkan things
    }
}
