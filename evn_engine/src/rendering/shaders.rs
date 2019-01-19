use hashbrown::HashMap;

// Will be storing compiled shaders down the line
pub struct ShaderMap(pub HashMap<String, (&'static [u8], &'static [u8])>);
