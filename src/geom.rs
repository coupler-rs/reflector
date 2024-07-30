pub use graphics::Point;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    #[inline]
    pub fn new(width: f32, height: f32) -> Size {
        Size { width, height }
    }

    #[inline]
    pub fn grow(self, width: f32, height: f32) -> Size {
        Size {
            width: self.width + width,
            height: self.height + height,
        }
    }

    #[inline]
    pub fn shrink(self, width: f32, height: f32) -> Size {
        Size {
            width: (self.width - width).max(0.0),
            height: (self.height - height).max(0.0),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ProposedSize {
    pub width: Option<f32>,
    pub height: Option<f32>,
}

impl From<Size> for ProposedSize {
    #[inline]
    fn from(value: Size) -> ProposedSize {
        ProposedSize {
            width: Some(value.width),
            height: Some(value.height),
        }
    }
}

impl ProposedSize {
    #[inline]
    pub fn new(width: Option<f32>, height: Option<f32>) -> ProposedSize {
        ProposedSize { width, height }
    }

    #[inline]
    pub fn unwrap_or(self, default: Size) -> Size {
        Size {
            width: self.width.unwrap_or(default.width),
            height: self.height.unwrap_or(default.height),
        }
    }

    #[inline]
    pub fn grow(self, width: f32, height: f32) -> ProposedSize {
        ProposedSize {
            width: self.width.map(|w| w + width),
            height: self.height.map(|h| h + height),
        }
    }

    #[inline]
    pub fn shrink(self, width: f32, height: f32) -> ProposedSize {
        ProposedSize {
            width: self.width.map(|w| (w - width).max(0.0)),
            height: self.height.map(|h| (h - height).max(0.0)),
        }
    }
}
