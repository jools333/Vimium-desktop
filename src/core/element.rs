#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    pub fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        self.width > 2 && self.height > 2
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UIElement {
    pub id: usize,
    pub name: String,
    pub role: String,
    pub rect: Rect,
}

impl UIElement {
    pub fn new(id: usize, name: impl Into<String>, role: impl Into<String>, rect: Rect) -> Self {
        Self {
            id,
            name: name.into(),
            role: role.into(),
            rect,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HintItem {
    pub hint: String,
    pub element: UIElement,
}
