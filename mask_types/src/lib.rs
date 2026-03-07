use std::sync::{Arc, RwLock};

#[allow(dead_code)]
#[derive(Debug)]
pub struct Script {
    // The code at the beginning of a Markdown's BlockCode
    pub lang_code: String,
    // The content at the beginning of a Markdown's BlockCode
    pub content: String,
}

#[derive(Debug)]
pub struct MaskData {
    pub level: i32,
    pub scripts: Vec<Script>,
}
type MaskInner = Arc<RwLock<MaskData>>;
#[derive(Clone, Debug)]
pub struct Mask(pub MaskInner);

impl clap::builder::CommandExt for Mask {}
impl std::ops::Deref for Mask {
    type Target = MaskInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Mask {
    pub fn new(level: i32) -> Self {
        Self(Arc::new(RwLock::new(
            MaskData {
                level,
                scripts: Default::default(),
            }
            .into(),
        )))
    }
}
