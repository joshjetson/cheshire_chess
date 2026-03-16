use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub player_name: String,
    pub sound: SoundSettings,
    #[serde(skip)]
    pub save_path: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SoundSettings {
    pub enabled: bool,
    pub master_volume: f32,     // 0.0 - 1.0
    pub filter_cutoff: f32,     // Hz, low-pass filter cutoff (500-8000)
    pub events: EventSounds,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventSounds {
    pub login: SynthParams,
    pub exit: SynthParams,
    pub piece_move: SynthParams,
    pub capture: SynthParams,
    pub check: SynthParams,
    pub checkmate: SynthParams,
    pub wrong_move: SynthParams,
    pub correct: SynthParams,
    pub hint: SynthParams,
    pub tick: SynthParams,
    pub select: SynthParams,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SynthParams {
    pub waveform: Waveform,
    pub frequency: f32,         // Hz
    pub attack: f32,            // seconds
    pub decay: f32,             // seconds
    pub sustain: f32,           // 0.0 - 1.0
    pub release: f32,           // seconds
    pub volume: f32,            // 0.0 - 1.0
    pub lfo_rate: f32,          // Hz (0 = off)
    pub lfo_depth: f32,         // 0.0 - 1.0
    pub duration_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Waveform {
    Sine,
    Triangle,
    Sawtooth,
    Square,
}

impl Waveform {
    pub fn next(&self) -> Self {
        match self {
            Waveform::Sine => Waveform::Triangle,
            Waveform::Triangle => Waveform::Sawtooth,
            Waveform::Sawtooth => Waveform::Square,
            Waveform::Square => Waveform::Sine,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Waveform::Sine => "Sine",
            Waveform::Triangle => "Triangle",
            Waveform::Sawtooth => "Sawtooth",
            Waveform::Square => "Square",
        }
    }
}

impl SynthParams {
    #[allow(dead_code)]
    fn soft(freq: f32, dur: u64) -> Self {
        Self {
            waveform: Waveform::Sine,
            frequency: freq,
            attack: 0.08,
            decay: 0.1,
            sustain: 0.4,
            release: 0.15,
            volume: 0.25,
            lfo_rate: 0.0,
            lfo_depth: 0.0,
            duration_ms: dur,
        }
    }
}

impl Default for EventSounds {
    fn default() -> Self {
        Self {
            login: SynthParams { waveform: Waveform::Sine, frequency: 330.0, attack: 0.15, decay: 0.1, sustain: 0.3, release: 0.25, volume: 0.2, lfo_rate: 2.0, lfo_depth: 0.1, duration_ms: 600 },
            exit: SynthParams { waveform: Waveform::Sine, frequency: 280.0, attack: 0.08, decay: 0.15, sustain: 0.2, release: 0.2, volume: 0.15, lfo_rate: 0.0, lfo_depth: 0.0, duration_ms: 400 },
            piece_move: SynthParams { waveform: Waveform::Triangle, frequency: 180.0, attack: 0.01, decay: 0.08, sustain: 0.0, release: 0.05, volume: 0.2, lfo_rate: 0.0, lfo_depth: 0.0, duration_ms: 120 },
            capture: SynthParams { waveform: Waveform::Triangle, frequency: 140.0, attack: 0.01, decay: 0.12, sustain: 0.0, release: 0.08, volume: 0.25, lfo_rate: 0.0, lfo_depth: 0.0, duration_ms: 180 },
            check: SynthParams { waveform: Waveform::Sine, frequency: 440.0, attack: 0.03, decay: 0.08, sustain: 0.3, release: 0.1, volume: 0.2, lfo_rate: 0.0, lfo_depth: 0.0, duration_ms: 250 },
            checkmate: SynthParams { waveform: Waveform::Sine, frequency: 260.0, attack: 0.12, decay: 0.1, sustain: 0.5, release: 0.3, volume: 0.2, lfo_rate: 1.5, lfo_depth: 0.08, duration_ms: 900 },
            wrong_move: SynthParams { waveform: Waveform::Triangle, frequency: 110.0, attack: 0.05, decay: 0.1, sustain: 0.2, release: 0.1, volume: 0.2, lfo_rate: 6.0, lfo_depth: 0.3, duration_ms: 300 },
            correct: SynthParams { waveform: Waveform::Sine, frequency: 660.0, attack: 0.02, decay: 0.08, sustain: 0.2, release: 0.15, volume: 0.15, lfo_rate: 0.0, lfo_depth: 0.0, duration_ms: 250 },
            hint: SynthParams { waveform: Waveform::Sine, frequency: 500.0, attack: 0.1, decay: 0.1, sustain: 0.3, release: 0.2, volume: 0.12, lfo_rate: 3.0, lfo_depth: 0.15, duration_ms: 400 },
            tick: SynthParams { waveform: Waveform::Sine, frequency: 400.0, attack: 0.005, decay: 0.03, sustain: 0.0, release: 0.01, volume: 0.1, lfo_rate: 0.0, lfo_depth: 0.0, duration_ms: 40 },
            select: SynthParams { waveform: Waveform::Sine, frequency: 500.0, attack: 0.01, decay: 0.05, sustain: 0.1, release: 0.05, volume: 0.15, lfo_rate: 0.0, lfo_depth: 0.0, duration_ms: 120 },
        }
    }
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            master_volume: 0.6,
            filter_cutoff: 3000.0,
            events: EventSounds::default(),
        }
    }
}

impl Settings {
    pub fn load(data_dir: &Path) -> Self {
        let save_path = data_dir.join("settings.json");
        let name = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "Player".into());

        if let Ok(content) = fs::read_to_string(&save_path) {
            if let Ok(mut settings) = serde_json::from_str::<Settings>(&content) {
                settings.save_path = save_path;
                return settings;
            }
        }

        Self {
            player_name: name,
            sound: SoundSettings::default(),
            save_path,
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(parent) = self.save_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&self.save_path, json)
    }
}

// ── Settings UI state ──────────────────────────────────────────────

pub const SETTINGS_ITEMS: &[&str] = &[
    "Player Name",
    "Sound Settings",
    "Piece Canvas",
    "Back",
];

pub const SOUND_EVENT_NAMES: &[&str] = &[
    "Login", "Exit", "Piece Move", "Capture", "Check",
    "Checkmate", "Wrong Move", "Correct", "Hint", "Tick", "Select",
];

pub const SYNTH_PARAM_NAMES: &[&str] = &[
    "Waveform", "Frequency", "Attack", "Decay",
    "Sustain", "Release", "Volume", "LFO Rate", "LFO Depth", "Duration",
];
