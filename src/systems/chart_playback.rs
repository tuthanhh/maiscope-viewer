use super::component::{ChartEvent, TimedEvent};
use super::{DEFAULT_BPM, GROWING, MOVING};
use bevy::prelude::*;

#[derive(Resource)]
#[allow(dead_code)]
pub struct ChartPlayback {
    /// Pre-computed list of timed events with absolute timestamps.
    pub timed_events: Vec<TimedEvent>,
    /// Index of the next timed event to be spawned.
    pub next_spawn_index: usize,

    /// Speed of the note (visual travel speed multiplier).
    pub note_speed: f32,
    /// Playing speed (chart speed multiplier, affects both timing and visuals).
    pub chart_speed: f32,
    /// Is playing.
    pub is_playing: bool,
    /// Elapsed playback time in seconds (advances each frame).
    pub elapsed_time: f64,
}

impl Default for ChartPlayback {
    fn default() -> Self {
        Self {
            timed_events: Vec::new(),
            next_spawn_index: 0,
            note_speed: 7.0,
            chart_speed: 0.25,
            is_playing: true,
            elapsed_time: 0.0,
        }
    }
}

impl ChartPlayback {
    /// Pre-compute absolute timestamps for every event in the parsed chart.
    ///
    /// The simai timing model:
    /// - `(BPM)` sets the current BPM.
    /// - `{N}` sets the length divider (resolution).
    /// - `{#S}` sets the per-comma length directly to S seconds (absolute length mode).
    /// - Per-comma length = `240 / BPM / resolution` (seconds), unless absolute length is active.
    /// - Each comma (`Rest` or `NoteGroup`) advances time by the per-comma length.
    /// - BPM/resolution changes take effect immediately and do NOT advance time.

    pub fn compute_timestamps(&mut self, events: Vec<ChartEvent>) {
        let mut timed: Vec<TimedEvent> = Vec::with_capacity(events.len());
        let mut current_time: f64 = 0.0;
        let mut bpm: f32 = 240.0;
        let mut resolution: u32 = 4;
        // When `{#S}` is active, per-comma length is set directly in seconds.
        // A subsequent `{N}` (resolution change) clears this and reverts to
        // the normal `240 / BPM / resolution` formula.
        let mut absolute_comma_length: Option<f64> = None;

        for event in events {
            match &event {
                ChartEvent::BpmChange(new_bpm) => {
                    bpm = *new_bpm;
                }
                ChartEvent::ResolutionChange(new_res) => {
                    resolution = *new_res;
                    absolute_comma_length = None;
                }
                ChartEvent::AbsoluteLength(seconds) => {
                    absolute_comma_length = Some(*seconds);
                }
                ChartEvent::NoteGroup(_) | ChartEvent::Rest => {
                    timed.push(TimedEvent {
                        time: current_time,
                        event: event.clone(),
                        bpm,
                    });
                    let per_comma = absolute_comma_length
                        .unwrap_or_else(|| DEFAULT_BPM as f64 / bpm as f64 / resolution as f64);
                    current_time += per_comma;
                }
            }
        }

        self.timed_events = timed;
        self.next_spawn_index = 0;
    }
    /// How far in advance (in seconds) notes should be spawned before their hit time.
    /// Equals `growing_time + move_duration` so the note arrives at the
    /// judgment line exactly at its scheduled hit time.
    pub fn approach_time(&self) -> f64 {
        let speed = (self.chart_speed * self.note_speed) as f64;
        (GROWING + MOVING) / speed
    }

    /// Checks whether the next event is due based on the current elapsed time,
    /// and if so, advances the spawn index and returns it.
    ///
    /// This is the real-time playback path — `elapsed_time` is NOT mutated here;
    /// it is driven externally by the frame loop.
    pub fn advance(&mut self) -> Option<TimedEvent> {
        let event = self.timed_events.get(self.next_spawn_index)?;

        if self.elapsed_time + self.approach_time() >= event.time {
            self.next_spawn_index += 1;
            Some(event.clone())
        } else {
            None
        }
    }
}
