use std::time::{Duration, Instant};

use easing_function::{Easing, EasingFunction};

use crate::color::{Lerp, TextColor};
use crate::component::DrawContext;
use crate::geometry::Rectangle;

#[derive(Debug)]
pub struct BlendAnimationDescriptor {
    pub easing_function: EasingFunction,
    pub start_delay: Duration,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct BlendAnimationProgress {
    pub instant_start: Instant,
    pub instant_end: Instant,
}

#[derive(Debug)]
pub struct BlendAnimation {
    pub descriptor: BlendAnimationDescriptor,
    pub progress: Option<BlendAnimationProgress>,
}

impl BlendAnimation {
    pub fn new_stopped(descriptor: BlendAnimationDescriptor) -> Self {
        Self {
            descriptor,
            progress: None,
        }
    }

    pub fn new_started(descriptor: BlendAnimationDescriptor, now: Instant) -> Self {
        let mut result = Self {
            descriptor,
            progress: None,
        };
        result.restart(now);
        result
    }

    pub fn restart(&mut self, now: Instant) {
        let instant_start = now + self.descriptor.start_delay;
        let instant_end = instant_start + self.descriptor.duration;
        self.progress = Some(BlendAnimationProgress {
            instant_start,
            instant_end,
        });
    }

    pub fn apply<T: Lerp + Clone>(&self, now: Instant, original: &T, new: &T) -> T {
        let Some(progress) = self.progress.as_ref() else {
            return new.clone();
        };

        if now <= progress.instant_start {
            original.clone()
        } else if now >= progress.instant_end {
            new.clone()
        } else {
            let period = progress
                .instant_end
                .duration_since(progress.instant_start)
                .as_secs_f32();
            let elapsed = now.duration_since(progress.instant_start).as_secs_f32();
            let normalized = elapsed / period;
            let eased = self.descriptor.easing_function.ease(normalized);

            Lerp::lerp(original, new, eased)
        }
    }
}

#[derive(Debug)]
pub enum RectAnimation {
    #[expect(unused)]
    Static { color: TextColor },
    ProgressIndeterminate {
        period: Duration,
        highlight: TextColor,
    },
    Ease {
        blend: BlendAnimation,
        color_start: TextColor,
        color_end: TextColor,
    },
}

impl RectAnimation {
    pub fn apply(&self, context: &mut DrawContext, area: Rectangle<i16>) {
        match self {
            RectAnimation::Static { color } => {
                context.set_style(area, color);
            }
            RectAnimation::ProgressIndeterminate { period, highlight } => {
                let cos = (context.elapsed_time().as_secs_f32() * std::f32::consts::TAU
                    / period.as_secs_f32())
                .cos();
                let highlight_index =
                    (0.5 * (1.0 + cos) * area.extent().x.saturating_sub(1) as f32 + 0.5) as i16;
                let position = [area.min().x + highlight_index, area.min().y];

                if let Some(cell) = context.get_cell_mut(position) {
                    cell.set_style(highlight);
                }
            }
            RectAnimation::Ease {
                blend,
                color_start,
                color_end,
            } => {
                let style = blend.apply(context.now(), color_start, color_end);

                context.set_style(area, style);
            }
        }
    }
}
