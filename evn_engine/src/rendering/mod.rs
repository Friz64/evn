pub mod shaders;

use specs::System;
use winit::Window;

pub struct Renderer {
    pub window: Window,
}

impl Renderer {
    pub fn new(window: Window) -> Self {
        Renderer { window }
    }
}

impl<'a> System<'a> for Renderer {
    type SystemData = ();

    // drawing
    fn run(&mut self, (): Self::SystemData) {}
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // destory all vulkan things
    }
}
