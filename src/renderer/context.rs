use glam::Mat4;

pub static DEFAULT_CONTEXT: Context = Context {
    transform: Mat4::IDENTITY,
};

#[derive(Debug, Clone)]
pub struct Context {
    transform: Mat4,
}

impl Default for Context {
    fn default() -> Self {
        DEFAULT_CONTEXT.clone()
    }
}

impl Context {
    pub fn transform(&self) -> &Mat4 {
        &self.transform
    }

    pub fn add_transform(&self, transform: &Mat4) -> Self {
        Self {
            transform: self.transform * (*transform),
        }
    }
}
