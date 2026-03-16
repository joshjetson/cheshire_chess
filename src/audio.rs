#[cfg(feature = "audio")]
use std::f32::consts::PI;

#[cfg(feature = "audio")]
use rodio::{OutputStream, OutputStreamHandle, Source, buffer::SamplesBuffer};

use crate::settings::{SynthParams, SoundSettings, Waveform};

#[cfg(feature = "audio")]
pub struct Audio {
    _stream: OutputStream,
    handle: OutputStreamHandle,
}

#[cfg(not(feature = "audio"))]
pub struct Audio;

#[cfg(feature = "audio")]
const SR: u32 = 44100;
#[cfg(feature = "audio")]
const SRF: f32 = 44100.0;

#[cfg(feature = "audio")]
fn osc(t: f32, freq: f32, waveform: &Waveform) -> f32 {
    match waveform {
        Waveform::Sine => (2.0 * PI * freq * t).sin(),
        Waveform::Triangle => {
            let p = (t * freq).fract();
            if p < 0.25 { p * 4.0 }
            else if p < 0.75 { 2.0 - p * 4.0 }
            else { p * 4.0 - 4.0 }
        }
        Waveform::Sawtooth => 2.0 * (t * freq).fract() - 1.0,
        Waveform::Square => if (t * freq).fract() < 0.5 { 0.8 } else { -0.8 },
    }
}

#[cfg(feature = "audio")]
fn adsr(t: f32, a: f32, d: f32, s: f32, r: f32, total: f32) -> f32 {
    let release_start = total - r;
    if t < a {
        // Attack
        t / a
    } else if t < a + d {
        // Decay
        let decay_t = (t - a) / d;
        1.0 - decay_t * (1.0 - s)
    } else if t < release_start {
        // Sustain
        s
    } else if t < total {
        // Release
        let release_t = (t - release_start) / r;
        s * (1.0 - release_t)
    } else {
        0.0
    }
}

#[cfg(feature = "audio")]
struct LowPass {
    prev: f32,
    alpha: f32,
}

#[cfg(feature = "audio")]
impl LowPass {
    fn new(cutoff: f32) -> Self {
        let rc = 1.0 / (2.0 * PI * cutoff);
        let dt = 1.0 / SRF;
        Self { prev: 0.0, alpha: dt / (rc + dt) }
    }

    fn process(&mut self, sample: f32) -> f32 {
        self.prev += self.alpha * (sample - self.prev);
        self.prev
    }
}

#[cfg(feature = "audio")]
pub fn render_sound(params: &SynthParams, master_vol: f32, filter_cutoff: f32) -> Vec<f32> {
    let samples = (SR as u64 * params.duration_ms) / 1000;
    let total_time = params.duration_ms as f32 / 1000.0;
    let mut filter = LowPass::new(filter_cutoff);
    let mut buf = Vec::with_capacity(samples as usize);

    for i in 0..samples {
        let t = i as f32 / SRF;

        // LFO modulation
        let lfo = if params.lfo_rate > 0.0 {
            1.0 + (2.0 * PI * params.lfo_rate * t).sin() * params.lfo_depth
        } else {
            1.0
        };

        let freq = params.frequency * lfo;
        let raw = osc(t, freq, &params.waveform);
        let env = adsr(t, params.attack, params.decay, params.sustain, params.release, total_time);
        let sample = raw * env * params.volume * master_vol;
        let filtered = filter.process(sample);

        buf.push(filtered.clamp(-1.0, 1.0));
    }

    buf
}

#[cfg(feature = "audio")]
fn play_buf(handle: &OutputStreamHandle, buf: Vec<f32>) {
    let source = SamplesBuffer::new(1, SR, buf);
    let _ = handle.play_raw(source.convert_samples());
}

#[cfg(feature = "audio")]
impl Audio {
    pub fn new() -> Option<Self> {
        // Suppress ALSA/audio errors from corrupting the TUI
        // Redirect stderr temporarily during audio init
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::io::AsRawFd;
            unsafe {
                let devnull = libc::open(b"/dev/null\0".as_ptr() as *const _, libc::O_WRONLY);
                if devnull >= 0 {
                    let saved = libc::dup(2);
                    libc::dup2(devnull, 2);
                    let result = OutputStream::try_default();
                    libc::dup2(saved, 2);
                    libc::close(devnull);
                    libc::close(saved);
                    let (stream, handle) = result.ok()?;
                    return Some(Self { _stream: stream, handle });
                }
            }
        }
        let (stream, handle) = OutputStream::try_default().ok()?;
        Some(Self { _stream: stream, handle })
    }

    pub fn play(&self, params: &SynthParams, sound: &SoundSettings) {
        if !sound.enabled { return; }
        let buf = render_sound(params, sound.master_volume, sound.filter_cutoff);
        play_buf(&self.handle, buf);
    }

    // Convenience methods that take the full SoundSettings
    pub fn play_login(&self, s: &SoundSettings) { self.play(&s.events.login, s); }
    pub fn play_exit(&self, s: &SoundSettings) { self.play(&s.events.exit, s); }
    pub fn play_move(&self, s: &SoundSettings) { self.play(&s.events.piece_move, s); }
    pub fn play_capture(&self, s: &SoundSettings) { self.play(&s.events.capture, s); }
    pub fn play_check(&self, s: &SoundSettings) { self.play(&s.events.check, s); }
    pub fn play_checkmate(&self, s: &SoundSettings) { self.play(&s.events.checkmate, s); }
    pub fn play_wrong(&self, s: &SoundSettings) { self.play(&s.events.wrong_move, s); }
    pub fn play_correct(&self, s: &SoundSettings) { self.play(&s.events.correct, s); }
    pub fn play_hint(&self, s: &SoundSettings) { self.play(&s.events.hint, s); }
    #[allow(dead_code)]
    pub fn play_tick(&self, s: &SoundSettings) { self.play(&s.events.tick, s); }
    #[allow(dead_code)]
    pub fn play_select(&self, s: &SoundSettings) { self.play(&s.events.select, s); }

    /// Ascending arpeggio for session/puzzle complete — uses the correct sound params
    pub fn play_session_complete(&self, s: &SoundSettings) {
        // Play a modified version of the correct sound with longer duration
        let mut params = s.events.correct.clone();
        params.duration_ms = 600;
        params.attack = 0.1;
        params.release = 0.3;
        self.play(&params, s);
    }
}

#[cfg(not(feature = "audio"))]
impl Audio {
    pub fn new() -> Option<Self> { Some(Audio) }
    pub fn play(&self, _params: &SynthParams, _sound: &SoundSettings) {}
    pub fn play_login(&self, _s: &SoundSettings) {}
    pub fn play_exit(&self, _s: &SoundSettings) {}
    pub fn play_move(&self, _s: &SoundSettings) {}
    pub fn play_capture(&self, _s: &SoundSettings) {}
    pub fn play_check(&self, _s: &SoundSettings) {}
    pub fn play_checkmate(&self, _s: &SoundSettings) {}
    pub fn play_wrong(&self, _s: &SoundSettings) {}
    pub fn play_correct(&self, _s: &SoundSettings) {}
    pub fn play_hint(&self, _s: &SoundSettings) {}
    pub fn play_tick(&self, _s: &SoundSettings) {}
    pub fn play_select(&self, _s: &SoundSettings) {}
    pub fn play_session_complete(&self, _s: &SoundSettings) {}
}
