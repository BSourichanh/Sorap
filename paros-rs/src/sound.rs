#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

const RATE: u32 = 22050;
const LEVEL: f32 = 0.22;
const PLAYERS: [&str; 3] = ["pw-play", "paplay", "aplay"];

static SYNTHESIZED_VOLUME: AtomicI32 = AtomicI32::new(-1);
static DETECTED_PLAYER: OnceLock<Option<&'static str>> = OnceLock::new();
static INIT_SIGNAL: std::sync::Once = std::sync::Once::new();

pub fn ensure_signal_handling() {
    INIT_SIGNAL.call_once(|| unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_flags = libc::SA_NOCLDWAIT | libc::SA_RESTART;
        libc::sigaction(libc::SIGCHLD, &sa, std::ptr::null_mut());
    });
}

pub fn reap_zombies() {
    ensure_signal_handling();
    unsafe {
        let mut status = 0;
        while libc::waitpid(-1, &mut status, libc::WNOHANG) > 0 {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundEffect {
    Success,
    Failure,
    Thud,
    Knock,
}

impl SoundEffect {
    pub const ALL: [SoundEffect; 4] = [
        SoundEffect::Success,
        SoundEffect::Failure,
        SoundEffect::Thud,
        SoundEffect::Knock,
    ];

    pub fn file_name(&self) -> &'static str {
        match self {
            SoundEffect::Success => "success",
            SoundEffect::Failure => "failure",
            SoundEffect::Thud => "thud",
            SoundEffect::Knock => "knock",
        }
    }

    fn tune(&self) -> &'static [(f32, &'static [f32])] {
        match self {
            SoundEffect::Success => &[(0.09, &[660.0]), (0.16, &[990.0])],
            SoundEffect::Failure => &[(0.12, &[330.0]), (0.24, &[220.0, 233.0])],
            SoundEffect::Thud => &[(0.08, &[0.0])],
            SoundEffect::Knock => &[(0.05, &[140.0, 0.0]), (0.16, &[]), (0.05, &[140.0, 0.0])],
        }
    }
}

pub fn runtime_sound_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("paros")
}

pub fn wav_path(effect: SoundEffect) -> PathBuf {
    runtime_sound_dir().join(format!("{}.wav", effect.file_name()))
}

pub fn synthesize(effect: SoundEffect, volume: i32) -> Vec<f32> {
    let gain = (volume.clamp(0, 100) as f32 / 100.0) * LEVEL;
    let mut samples = Vec::new();

    for &(duration, voices) in effect.tune() {
        let frames = (duration * RATE as f32) as usize;
        if frames == 0 {
            continue;
        }
        let voice_count = voices.len().max(1) as f32;
        for i in 0..frames {
            if voices.is_empty() {
                samples.push(0.0);
                continue;
            }
            let mut sample = 0.0f32;
            for &freq in voices {
                if freq <= 0.0 {
                    // Bruit blanc pour freq == 0.0
                    sample += fastrand::f32() * 2.0 - 1.0;
                } else {
                    // Onde carrée pure
                    let phase = (i as f32 * freq / RATE as f32).fract();
                    sample += if phase < 0.5 { 1.0 } else { -1.0 };
                }
            }
            // Enveloppe d'amplitude : décroissance linéaire pure
            let decay = 1.0 - (i as f32 / frames as f32);
            samples.push((sample / voice_count) * gain * decay);
        }
    }

    samples
}

pub fn to_wav(samples: &[f32]) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut buf = Vec::with_capacity(44 + data_len);

    // En-tête RIFF/WAVE 16-bit mono PCM à 22050 Hz (44 octets)
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_len as u32).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // Taille du bloc fmt
    buf.extend_from_slice(&1u16.to_le_bytes()); // Format PCM (1)
    buf.extend_from_slice(&1u16.to_le_bytes()); // Canaux : 1 (mono)
    buf.extend_from_slice(&RATE.to_le_bytes()); // Fréquence d'échantillonnage (22050 Hz)
    buf.extend_from_slice(&(RATE * 2).to_le_bytes()); // Octets par seconde
    buf.extend_from_slice(&2u16.to_le_bytes()); // Octets par trame (block align)
    buf.extend_from_slice(&16u16.to_le_bytes()); // Bits par échantillon (16)
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&(data_len as u32).to_le_bytes());

    for &s in samples {
        let pcm = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        buf.extend_from_slice(&pcm.to_le_bytes());
    }

    buf
}

pub fn ensure_wav_files(volume: i32) {
    let clamped = volume.clamp(0, 100);
    let dir = runtime_sound_dir();
    let all_exist = SoundEffect::ALL.iter().all(|e| wav_path(*e).exists());

    if SYNTHESIZED_VOLUME.load(Ordering::Relaxed) == clamped && all_exist {
        return;
    }

    if fs::create_dir_all(&dir).is_ok() {
        for effect in SoundEffect::ALL {
            let wav = to_wav(&synthesize(effect, clamped));
            let _ = fs::write(wav_path(effect), wav);
        }
        SYNTHESIZED_VOLUME.store(clamped, Ordering::Relaxed);
    }
}

fn find_player() -> Option<&'static str> {
    *DETECTED_PLAYER.get_or_init(|| {
        let path_var = std::env::var_os("PATH")?;
        for player in PLAYERS {
            for dir in std::env::split_paths(&path_var) {
                let candidate = dir.join(player);
                if candidate.is_file() {
                    return Some(player);
                }
            }
        }
        None
    })
}

pub fn play_sound_with_settings(effect: SoundEffect, enabled: bool, volume: i32) {
    if !enabled || volume <= 0 {
        return;
    }
    reap_zombies();
    ensure_wav_files(volume);
    let Some(player) = find_player() else {
        return;
    };
    let path = wav_path(effect);
    if path.exists()
        && let Ok(mut child) = Command::new(player).arg(path).spawn()
    {
        let _ = child.try_wait();
    }
}

pub fn play_sound(effect: SoundEffect) {
    let prev_vol = SYNTHESIZED_VOLUME.load(Ordering::Relaxed);
    let volume = if prev_vol >= 0 { prev_vol } else { 50 };
    play_sound_with_settings(effect, true, volume);
}
