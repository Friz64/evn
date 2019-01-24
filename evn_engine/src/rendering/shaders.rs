#[derive(Debug)]
pub struct Shader {
    // will be stored differently when the renderer is implemented
    pub vert: Vec<u8>,
    pub frag: Vec<u8>,
}
