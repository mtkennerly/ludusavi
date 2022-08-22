use iced::{Length, ProgressBar};

#[derive(Default)]
pub struct DisappearingProgress {
    pub max: f32,
    pub current: f32,
}

impl DisappearingProgress {
    pub fn view(&mut self) -> ProgressBar {
        let visible = self.current > 0.0 && self.current < self.max;
        ProgressBar::new(0.0..=self.max, self.current).height(Length::FillPortion(if visible { 100 } else { 1 }))
    }
}
