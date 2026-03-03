//! Live audio spectrum visualization — WAV file playback with FFT → 3D point cloud.
//!
//! Run: cargo run --example audio -- path/to/file.wav
//!
//! Controls:
//!   - Orbit camera with mouse
//!   - Press S to take a screenshot
//!   - Press Space to pause/resume

use std::sync::{Arc, Mutex};

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use bevy_point_cloud::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::{FftPlanner, num_complex::Complex};

const FFT_SIZE: usize = 2048;
const SPECTRUM_BINS: usize = FFT_SIZE / 2;
const HISTORY_ROWS: usize = 120;

/// Shared ring buffer: audio thread writes samples, main thread reads for FFT.
struct AudioRing {
    buffer: Vec<f32>,
    write_pos: usize,
    paused: bool,
}

/// Send+Sync wrapper around the shared ring buffer.
#[derive(Resource, Clone)]
struct SharedRing(Arc<Mutex<AudioRing>>);

/// Holds the cpal stream (which is !Send). Inserted as NonSend resource.
struct AudioStream(#[allow(dead_code)] cpal::Stream);

#[derive(Resource)]
struct SpectrumState {
    history: Vec<Vec<f32>>,
    fft_scratch: Vec<Complex<f32>>,
    window: Vec<f32>,
    planner: FftPlanner<f32>,
}

fn main() {
    let wav_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run --example audio -- path/to/file.wav");
        std::process::exit(1);
    });

    // Load WAV file
    let reader = hound::WavReader::open(&wav_path).unwrap_or_else(|e| {
        eprintln!("Failed to open {wav_path}: {e}");
        std::process::exit(1);
    });
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels as usize;

    // Decode all samples to mono f32
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1u32 << (spec.bits_per_sample - 1)) as f32;
            reader
                .into_samples::<i32>()
                .filter_map(|s| s.ok())
                .collect::<Vec<_>>()
                .chunks(channels)
                .map(|frame| frame.iter().sum::<i32>() as f32 / (max_val * channels as f32))
                .collect()
        }
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .filter_map(|s| s.ok())
            .collect::<Vec<_>>()
            .chunks(channels)
            .map(|frame| frame.iter().sum::<f32>() / channels as f32)
            .collect(),
    };

    eprintln!(
        "Loaded {wav_path}: {} samples, {}Hz, {}ch, {:.1}s",
        samples.len(),
        sample_rate,
        channels,
        samples.len() as f32 / sample_rate as f32
    );

    // Shared ring buffer
    let shared_ring = SharedRing(Arc::new(Mutex::new(AudioRing {
        buffer: vec![0.0; FFT_SIZE * 4],
        write_pos: 0,
        paused: false,
    })));

    // Set up cpal output stream
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No audio output device");
    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(sample_rate),
        buffer_size: cpal::BufferSize::Default,
    };

    let ring_for_audio = shared_ring.0.clone();
    let samples = Arc::new(samples);
    let playback_pos = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let playback_pos_clone = playback_pos.clone();
    let samples_clone = samples.clone();

    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut ring = ring_for_audio.lock().unwrap();
                let paused = ring.paused;
                let buf_len = ring.buffer.len();

                for frame in data.chunks_mut(2) {
                    let pos = playback_pos_clone.load(std::sync::atomic::Ordering::Relaxed);
                    let sample = if !paused && pos < samples_clone.len() {
                        let s = samples_clone[pos];
                        playback_pos_clone.store(pos + 1, std::sync::atomic::Ordering::Relaxed);
                        s
                    } else {
                        0.0
                    };

                    frame[0] = sample;
                    frame[1] = sample;

                    let wp = ring.write_pos % buf_len;
                    ring.buffer[wp] = sample;
                    ring.write_pos += 1;
                }
            },
            |err| eprintln!("Audio stream error: {err}"),
            None,
        )
        .expect("Failed to build audio stream");

    stream.play().expect("Failed to start audio stream");

    // Hann window
    let window: Vec<f32> = (0..FFT_SIZE)
        .map(|i| {
            let t = i as f32 / (FFT_SIZE - 1) as f32;
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * t).cos())
        })
        .collect();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "bevy_point_cloud — audio spectrum".into(),
                resolution: bevy::window::WindowResolution::new(1280, 720),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(PointCloudPlugin)
        .add_plugins(PanOrbitCameraPlugin)
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(shared_ring)
        .insert_non_send_resource(AudioStream(stream))
        .insert_resource(SpectrumState {
            history: vec![vec![0.0; SPECTRUM_BINS]; HISTORY_ROWS],
            fft_scratch: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            window,
            planner: FftPlanner::new(),
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (update_spectrum, take_screenshot, toggle_pause))
        .run();
}

#[derive(Component)]
struct SpectrumCloud;

fn setup(mut commands: Commands) {
    commands.spawn((SpectrumCloud, PointCloud::new(vec![])));

    commands.spawn((
        Camera3d::default(),
        Tonemapping::None,
        Transform::from_xyz(0.0, 8.0, 25.0).looking_at(Vec3::new(0.0, 4.0, 0.0), Vec3::Y),
        PanOrbitCamera {
            target_focus: Vec3::new(0.0, 4.0, 0.0),
            ..default()
        },
        NoIndirectDrawing,
    ));
}

fn update_spectrum(
    ring: Res<SharedRing>,
    mut spectrum: ResMut<SpectrumState>,
    mut clouds: Query<&mut PointCloud, With<SpectrumCloud>>,
) {
    // Read latest FFT_SIZE samples from ring buffer
    {
        let ring = ring.0.lock().unwrap();
        let buf_len = ring.buffer.len();
        let wp = ring.write_pos;
        for i in 0..FFT_SIZE {
            let idx = (wp + buf_len - FFT_SIZE + i) % buf_len;
            let windowed = ring.buffer[idx] * spectrum.window[i];
            spectrum.fft_scratch[i] = Complex::new(windowed, 0.0);
        }
    }

    // FFT
    let fft = spectrum.planner.plan_fft_forward(FFT_SIZE);
    fft.process(&mut spectrum.fft_scratch);

    // Magnitude spectrum (first half)
    let mut magnitudes = vec![0.0f32; SPECTRUM_BINS];
    for (i, mag) in magnitudes.iter_mut().enumerate() {
        let m = spectrum.fft_scratch[i].norm();
        *mag = (1.0 + m).ln().min(5.0) / 5.0;
    }

    // Shift history and push new row
    spectrum.history.remove(0);
    spectrum.history.push(magnitudes);

    // Rebuild point cloud
    let Ok(mut cloud) = clouds.single_mut() else {
        return;
    };

    let width = 20.0;
    let depth = 15.0;
    let height_scale = 12.0;

    cloud.points.clear();

    for (row_idx, row) in spectrum.history.iter().enumerate() {
        let z = (row_idx as f32 / HISTORY_ROWS as f32 - 0.5) * depth;
        let row_age = row_idx as f32 / HISTORY_ROWS as f32;

        for (bin_idx, &mag) in row.iter().enumerate() {
            if mag < 0.01 {
                continue;
            }

            // Log-scale frequency axis
            let freq_frac = (bin_idx as f32 + 1.0).ln() / (SPECTRUM_BINS as f32).ln();
            let x = (freq_frac - 0.5) * width;
            let y = mag * height_scale;

            let brightness = 0.3 + 0.7 * mag;
            let age_alpha = 0.1 + 0.9 * row_age;
            let (r, g, b) = hsv_to_rgb(freq_frac * 0.7, 0.6, brightness);

            cloud.points.push(PointData::new(
                Vec3::new(x, y, z),
                1.2 + mag * 2.0,
                Vec4::new(r, g, b, age_alpha * mag.sqrt()),
            ));
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

fn take_screenshot(mut commands: Commands, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::KeyS) {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk("/tmp/point_cloud_audio.png"));
        info!("Screenshot → /tmp/point_cloud_audio.png");
    }
}

fn toggle_pause(ring: Res<SharedRing>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Space) {
        let mut ring = ring.0.lock().unwrap();
        ring.paused = !ring.paused;
        info!(
            "Playback {}",
            if ring.paused { "paused" } else { "resumed" }
        );
    }
}
