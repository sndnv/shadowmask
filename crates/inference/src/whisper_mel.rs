use mel_spec::mel::{log_mel_spectrogram, mel};
use mel_spec::stft::Spectrogram;
use ndarray::Array2;

use crate::audio::SAMPLE_RATE;
use crate::whisper_loop::WINDOW_FRAMES;

const FFT_SIZE: usize = 400;
const HOP: usize = 160;
const LEAD: isize = 280;
const DYNAMIC_RANGE: f64 = 8.0;

pub struct Features<'a> {
    samples: &'a [f32],
    filters: Array2<f64>,
    floor: f64,
}

impl<'a> Features<'a> {
    pub fn new(samples: &'a [f32], mels: usize) -> Self {
        let filters = mel(f64::from(SAMPLE_RATE), FFT_SIZE, mels, None, None, false, true);
        let mut features = Self { samples, filters, floor: f64::NEG_INFINITY };
        let peak = (0..features.frames())
            .step_by(WINDOW_FRAMES)
            .map(|start| {
                features.log_mel(start).fold(f64::NEG_INFINITY, |peak, value| peak.max(*value))
            })
            .fold(f64::NEG_INFINITY, f64::max);
        features.floor = peak - DYNAMIC_RANGE;
        features
    }

    pub fn frames(&self) -> usize {
        self.samples.len().div_ceil(HOP)
    }

    pub fn window(&self, start_frame: usize) -> Vec<f32> {
        self.log_mel(start_frame)
            .iter()
            .map(|value| ((value.max(self.floor) + 4.0) / 4.0) as f32)
            .collect()
    }

    fn log_mel(&self, start_frame: usize) -> Array2<f64> {
        let mut stft = Spectrogram::new(FFT_SIZE, HOP);
        let mut frames = Array2::zeros((self.filters.nrows(), WINDOW_FRAMES));
        let mut from = (start_frame * HOP) as isize - LEAD;
        let mut column = 0;
        while column < WINDOW_FRAMES {
            let hop: Vec<f32> =
                (from..from + HOP as isize).map(|index| self.sample(index)).collect();
            if let Some(fft) = stft.add(&hop) {
                frames
                    .column_mut(column)
                    .assign(&log_mel_spectrogram(&fft, &self.filters).column(0));
                column += 1;
            }
            from += HOP as isize;
        }
        frames
    }

    fn sample(&self, index: isize) -> f32 {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.samples.get(index))
            .copied()
            .unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use super::*;

    const MELS: usize = 80;

    fn tone(seconds: f32, amplitude: f32) -> Vec<f32> {
        let rate = SAMPLE_RATE as f32;
        (0..(seconds * rate) as usize)
            .map(|index| amplitude * (2.0 * PI * 440.0 * index as f32 / rate).sin())
            .collect()
    }

    fn column(window: &[f32], frame: usize) -> Vec<f32> {
        (0..MELS).map(|mel| window[mel * WINDOW_FRAMES + frame]).collect()
    }

    fn energy(window: &[f32], frame: usize) -> f32 {
        column(window, frame).iter().sum()
    }

    fn loudest(window: &[f32]) -> usize {
        (0..WINDOW_FRAMES).max_by(|a, b| energy(window, *a).total_cmp(&energy(window, *b))).unwrap()
    }

    #[test]
    fn a_window_holds_three_thousand_frames_padded_with_silence() {
        let mut samples = tone(1.0, 0.5);
        samples.push(0.1);

        let features = Features::new(&samples, MELS);
        let window = features.window(0);

        assert_eq!(features.frames(), 101);
        assert_eq!(window.len(), MELS * WINDOW_FRAMES);
        let silent = column(&window, 2_000);
        assert!(silent.iter().all(|value| *value == silent[0]));
        assert!(energy(&window, 50) > energy(&window, 2_000));
    }

    #[test]
    fn no_audio_has_no_frames() {
        assert_eq!(Features::new(&[], MELS).frames(), 0);
    }

    #[test]
    fn silence_reads_the_same_everywhere() {
        let samples = vec![0.0; SAMPLE_RATE as usize];

        let window = Features::new(&samples, MELS).window(0);

        assert!(window.iter().all(|value| (value + 1.5).abs() < 1e-6));
    }

    #[test]
    fn a_burst_lands_in_the_frame_centred_on_it_in_any_window() {
        let mut samples = vec![0.0; 3 * SAMPLE_RATE as usize];
        for (offset, sample) in samples[15_920..16_080].iter_mut().enumerate() {
            *sample = 0.5 * (2.0 * PI * 1_000.0 * offset as f32 / SAMPLE_RATE as f32).sin();
        }

        let features = Features::new(&samples, MELS);
        let first = features.window(0);
        let later = features.window(40);

        assert_eq!(loudest(&first), 100);
        assert_eq!(loudest(&later), 60);
        for frame in 99..=101 {
            assert_eq!(column(&first, frame), column(&later, frame - 40));
        }
    }

    #[test]
    fn the_loudest_window_sets_the_floor_for_all() {
        let quiet = tone(30.0, 0.001);
        let mut mixed = quiet.clone();
        mixed.extend(tone(30.0, 0.5));
        let floor = |samples: &[f32]| {
            Features::new(samples, MELS).window(0).into_iter().fold(f32::INFINITY, f32::min)
        };

        assert!(floor(&mixed) > floor(&quiet));
    }
}
