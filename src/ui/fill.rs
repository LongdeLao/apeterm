use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

#[derive(Debug, Clone, Copy)]
pub struct Fill {
    color: Color,
}

impl Fill {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

impl Widget for Fill {
    fn render(self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, Style::default().bg(self.color));
    }
}
