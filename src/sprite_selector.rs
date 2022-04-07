use std::time::Duration;

use ahash::AHashMap;
use specs::{
    storage::{PairedStorage, SequentialRestriction},
    BitSet, Component, FlaggedStorage, Read, System, VecStorage, WriteStorage,
};

use crate::actor::Time;

#[derive(Debug, Clone)]
pub struct SpriteSelector {
    min: u32,
    max: u32,
    max_width: u32,
    on: bool,
    pub width: f32,
    pub height: f32,
    pub wait: Option<Duration>,
    pub time: Duration,
    pub at: u32,
}

impl Component for SpriteSelector {
    type Storage = FlaggedStorage<Self, VecStorage<Self>>;
}

impl SpriteSelector {
    pub fn new(
        start: u32,
        min: u32,
        max: u32,
        width: u32,
        height: u32,
        wait: Option<Duration>,
    ) -> Self {
        Self {
            min,
            max,
            max_width: width,
            on: false,
            width: 1.0 / width as f32,
            height: 1.0 / height as f32,
            wait,
            time: Duration::from_millis(0),
            at: start.clamp(min, max - 1),
        }
    }

    pub fn from_mat(unknown_param: &AHashMap<String, String>) -> Self {
        let default = String::new();

        let min = unknown_param
            .get("min")
            .unwrap_or(&default)
            .parse::<u32>()
            .unwrap_or(0);

        let start = unknown_param
            .get("start")
            .unwrap_or(&default)
            .parse::<u32>()
            .unwrap_or(min);

        let max = unknown_param
            .get("max")
            .unwrap_or(&default)
            .parse::<u32>()
            .unwrap_or(1);

        let width = unknown_param
            .get("width")
            .unwrap_or(&default)
            .parse::<u32>()
            .unwrap_or(1);

        let height = unknown_param
            .get("height")
            .unwrap_or(&default)
            .parse::<u32>()
            .unwrap_or(1);

        let wait = match unknown_param.get("wait").unwrap_or(&default).parse::<u64>() {
            Ok(wait) => Some(Duration::from_millis(wait)),
            Err(_) => None,
        };

        Self::new(start, min, max, width, height, wait)
    }

    pub fn play(&mut self) {
        if self.wait.is_some() {
            self.on = true;
        }
    }

    pub fn stop(&mut self) {
        self.on = false;
    }

    pub fn update<'a>(
        this: &mut PairedStorage<
            Self,
            &mut FlaggedStorage<Self, VecStorage<Self>>,
            &BitSet,
            SequentialRestriction,
        >,
    ) {
        if this.get_unchecked().on {
            match this.get_unchecked().wait {
                Some(wait) => {
                    let elapsed = this.get_unchecked().time.as_nanos();
                    let skip = (elapsed / wait.as_nanos()) as u32;

                    if skip > 0 {
                        let this = this.get_mut_unchecked();

                        this.at = ((this.at + skip) % this.max).max(this.min);
                        this.time -= wait * skip;
                    }
                }
                None => {}
            }
        }
    }

    pub fn add_time(&mut self, delta: Duration) {
        if self.on {
            self.time += delta;
        }
    }

    pub fn _set(&mut self, at: u32) -> Result<(), ()> {
        let at = at + self.min;

        if at < self.max {
            self.at = at;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn get_current(&self) -> [[f32; 2]; 4] {
        self.calculate(self.at)
    }

    pub fn _get(&self, at: u32) -> Result<[[f32; 2]; 4], ()> {
        let at = at + self.min;

        if at < self.max {
            Ok(self.calculate(at))
        } else {
            Err(())
        }
    }

    fn calculate(&self, at: u32) -> [[f32; 2]; 4] {
        let w: f32 = (at % self.max_width) as f32;
        let h: f32 = (at / self.max_width) as f32;

        let w_0: f32 = (w * self.width).min(1.0 - self.width);
        let h_0: f32 = (h * self.height).min(1.0 - self.width);

        let w_1: f32 = (w_0 + self.width).min(1.0);
        let h_1: f32 = (h_0 + self.height).min(1.0);

        [[w_1, h_1], [w_1, h_0], [w_0, h_0], [w_0, h_1]]
    }
}

pub struct SpriteSelectorUpdate;

impl<'a> System<'a> for SpriteSelectorUpdate {
    type SystemData = (WriteStorage<'a, SpriteSelector>, Read<'a, Time>);

    fn run(&mut self, (mut sprite_selectors, time): Self::SystemData) {
        use specs::Join;

        let time_delta = time.delta.as_nanos();

        if time_delta > 0 {
            sprite_selectors.set_event_emission(false);
            for sprite_selector in (&mut sprite_selectors).join() {
                sprite_selector.add_time(time.delta);
            }

            sprite_selectors.set_event_emission(true);
            for mut sprite_selector in (&mut sprite_selectors.restrict_mut()).join() {
                SpriteSelector::update(&mut sprite_selector);
            }
        }
    }
}
