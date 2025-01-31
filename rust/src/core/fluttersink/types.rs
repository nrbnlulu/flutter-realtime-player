use std::rc::Rc;

#[derive(Debug, Default, Copy, Clone)]
pub enum Orientation {
    #[default]
    Auto,
    Rotate0,
    Rotate90,
    Rotate180,
    Rotate270,
    FlipRotate0,
    FlipRotate90,
    FlipRotate180,
    FlipRotate270,
}

impl Orientation {
    pub fn from_tags(tags: &gst::TagListRef) -> Option<Orientation> {
        let orientation = tags
            .generic("image-orientation")
            .and_then(|v| v.get::<String>().ok())?;

        Some(match orientation.as_str() {
            "rotate-0" => Orientation::Rotate0,
            "rotate-90" => Orientation::Rotate90,
            "rotate-180" => Orientation::Rotate180,
            "rotate-270" => Orientation::Rotate270,
            "flip-rotate-0" => Orientation::FlipRotate0,
            "flip-rotate-90" => Orientation::FlipRotate90,
            "flip-rotate-180" => Orientation::FlipRotate180,
            "flip-rotate-270" => Orientation::FlipRotate270,
            _ => return None,
        })
    }

    pub fn is_flip_width_height(self) -> bool {
        matches!(
            self,
            Orientation::Rotate90
                | Orientation::Rotate270
                | Orientation::FlipRotate90
                | Orientation::FlipRotate270
        )
    }
}