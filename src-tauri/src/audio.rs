use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sherpa_onnx::{KeywordSpotter, KeywordSpotterConfig, OnlineStream};
use tauri::{AppHandle, Emitter, Manager, State};
use crate::local_store::SafeStoreExt;

use crate::speech::{
    CaptureOutcome, CommandCapture, SpeechError, SpeechErrorCode, SpeechTranscriber,
};

const SETTINGS_FILE: &str = "nova-settings.json";
const SELECTED_MICROPHONE_KEY: &str = "selectedMicrophone";
const INTERNAL_SAMPLE_RATE: u32 = 16_000;
const INTERNAL_CHANNELS: u16 = 1;
const LEVEL_WINDOW_SAMPLES: usize = 800;
const WAKE_FRAME_SAMPLES: usize = 480;
const WAKE_ENGINE_NAME: &str = "sherpa-onnx Zipformer KWS";
const WAKE_PHRASE: &str = "NOVA or Hey NOVA";
const WAKE_MODEL_DIRECTORY: &str = "resources/wake-word";
const ENCODER_FILE: &str = "encoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx";
const DECODER_FILE: &str = "decoder-epoch-12-avg-2-chunk-16-left-64.int8.onnx";
const JOINER_FILE: &str = "joiner-epoch-12-avg-2-chunk-16-left-64.int8.onnx";
const TOKENS_FILE: &str = "tokens.txt";
const KEYWORDS_FILE: &str = "keywords.txt";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneDevice {
    id: String,
    name: String,
    is_default: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneDeviceList {
    devices: Vec<MicrophoneDevice>,
    selected_device_id: Option<String>,
    internal_format: AudioFormat,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFormat {
    channels: u16,
    sample_rate: u32,
    sample_type: &'static str,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            channels: INTERNAL_CHANNELS,
            sample_rate: INTERNAL_SAMPLE_RATE,
            sample_type: "f32 PCM",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneTestSnapshot {
    status: MicrophoneStatus,
    level: f32,
    device_id: Option<String>,
    device_name: Option<String>,
    message: String,
    internal_format: AudioFormat,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum MicrophoneStatus {
    Idle,
    Listening,
    Disconnected,
    PermissionDenied,
    Unavailable,
    Error,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeEngineSnapshot {
    engine: &'static str,
    phrase: &'static str,
    status: WakeEngineStatus,
    message: String,
    device_name: Option<String>,
    input_level: f32,
    noise_floor: f32,
    sensitivity: crate::settings::WakeSensitivity,
    threshold: f32,
    last_detected_phrase: Option<String>,
    last_wake_confidence: Option<f32>,
    last_detection_millis: Option<u128>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum WakeEngineStatus {
    Listening,
    Paused,
    Disabled,
    Error,
}

impl WakeEngineSnapshot {
    fn disabled() -> Self {
        Self {
            engine: WAKE_ENGINE_NAME,
            phrase: WAKE_PHRASE,
            status: WakeEngineStatus::Disabled,
            message: "Wake detection is disabled because NOVA is off.".into(),
            device_name: None,
            input_level: 0.0,
            noise_floor: 0.0,
            sensitivity: crate::settings::WakeSensitivity::default(),
            threshold: crate::settings::WakeSensitivity::default().threshold(),
            last_detected_phrase: None,
            last_wake_confidence: None,
            last_detection_millis: None,
        }
    }
}

#[derive(Clone)]
struct WakeConfiguration {
    app: AppHandle,
    enabled: bool,
    paused: bool,
    sensitivity: crate::settings::WakeSensitivity,
    device_id: Option<String>,
    model_directory: Option<PathBuf>,
    speech_model_path: PathBuf,
}
impl MicrophoneTestSnapshot {
    fn idle() -> Self {
        Self {
            status: MicrophoneStatus::Idle,
            level: 0.0,
            device_id: None,
            device_name: None,
            message: "Microphone test is stopped.".into(),
            internal_format: AudioFormat::default(),
        }
    }
}

impl Default for MicrophoneTestSnapshot {
    fn default() -> Self {
        Self::idle()
    }
}

pub struct AudioService {
    command_tx: Mutex<Option<mpsc::Sender<AudioCommand>>>,
    snapshot: Arc<Mutex<MicrophoneTestSnapshot>>,
    wake_snapshot: Arc<Mutex<WakeEngineSnapshot>>,
}

enum AudioCommand {
    ConfigureWake {
        configuration: WakeConfiguration,
        reply: mpsc::Sender<WakeEngineSnapshot>,
    },
    WakeSamples(Vec<f32>),
    WakeStreamError(String),
    TranscriptionFinished {
        generation: u64,
        result: Result<crate::speech::SpeechTranscription, SpeechError>,
    },
    BeginFollowup {
        speech_generation: Option<u64>,
        reply: mpsc::Sender<Result<(), String>>,
    },
    CancelVoiceSession {
        reply: mpsc::Sender<()>,
    },
    Start {
        device_id: Option<String>,
        reply: mpsc::Sender<Result<MicrophoneTestSnapshot, String>>,
    },
    Stop {
        reply: mpsc::Sender<MicrophoneTestSnapshot>,
    },
}

impl Default for AudioService {
    fn default() -> Self {
        let snapshot = Arc::new(Mutex::new(MicrophoneTestSnapshot::idle()));
        let wake_snapshot = Arc::new(Mutex::new(WakeEngineSnapshot::disabled()));
        let (command_tx, command_rx) = mpsc::channel();
        let worker_snapshot = Arc::clone(&snapshot);
        let worker_wake_snapshot = Arc::clone(&wake_snapshot);
        let worker_tx = command_tx.clone();
        let transcriber = SpeechTranscriber::default();
        let worker = std::thread::Builder::new()
            .name("nova-audio-capture".into())
            .spawn(move || {
                audio_worker(
                    command_rx,
                    worker_tx,
                    worker_snapshot,
                    worker_wake_snapshot,
                    transcriber,
                )
            });

        let command_tx = match worker {
            Ok(_) => Some(command_tx),
            Err(error) => {
                set_snapshot(
                    &snapshot,
                    MicrophoneTestSnapshot {
                        status: MicrophoneStatus::Unavailable,
                        level: 0.0,
                        device_id: None,
                        device_name: None,
                        message: format!("Could not start the local audio service: {error}"),
                        internal_format: AudioFormat::default(),
                    },
                );
                None
            }
        };

        Self {
            command_tx: Mutex::new(command_tx),
            snapshot,
            wake_snapshot,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectMicrophoneArguments {
    device_id: String,
}

struct LevelMeter {
    source_rate: u32,
    source_channels: usize,
    resample_accumulator: u32,
    sum_squares: f64,
    sample_count: usize,
}

impl LevelMeter {
    fn new(source_rate: u32, source_channels: u16) -> Self {
        Self {
            source_rate: source_rate.max(1),
            source_channels: usize::from(source_channels.max(1)),
            resample_accumulator: 0,
            sum_squares: 0.0,
            sample_count: 0,
        }
    }

    fn consume<T>(
        &mut self,
        samples: &[T],
        convert: impl Fn(&T) -> f32,
        snapshot: &Arc<Mutex<MicrophoneTestSnapshot>>,
    ) {
        for frame in samples.chunks(self.source_channels) {
            let mono = frame.iter().map(&convert).sum::<f32>() / frame.len() as f32;
            self.resample_accumulator = self
                .resample_accumulator
                .saturating_add(INTERNAL_SAMPLE_RATE);

            while self.resample_accumulator >= self.source_rate {
                self.resample_accumulator -= self.source_rate;
                let sample = mono.clamp(-1.0, 1.0);
                self.sum_squares += f64::from(sample * sample);
                self.sample_count += 1;

                if self.sample_count >= LEVEL_WINDOW_SAMPLES {
                    let rms = (self.sum_squares / self.sample_count as f64).sqrt() as f32;
                    let level = (rms * 3.0).clamp(0.0, 1.0);
                    if let Ok(mut current) = snapshot.lock() {
                        current.level = level;
                    }
                    self.sum_squares = 0.0;
                    self.sample_count = 0;
                }
            }
        }
    }
}

fn permission_denied(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    normalized.contains("access denied")
        || normalized.contains("permission denied")
        || normalized.contains("not authorized")
        || normalized.contains("0x80070005")
}

fn slug(value: &str) -> String {
    let mut result = String::new();
    let mut separator = false;

    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            result.push(character);
            separator = false;
        } else if !separator && !result.is_empty() {
            result.push('-');
            separator = true;
        }
    }

    result.trim_end_matches('-').to_string()
}

fn enumerate_devices() -> Result<Vec<(MicrophoneDevice, Device)>, String> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());
    let devices = host
        .input_devices()
        .map_err(|error| format!("Could not enumerate microphone devices: {error}"))?;
    let mut result = Vec::new();
    let mut names = HashMap::<String, usize>::new();

    for device in devices {
        let name = device
            .name()
            .unwrap_or_else(|_| "Unknown microphone".to_string());
        let occurrence = names.entry(name.clone()).or_default();
        let id = format!("{}-{}", slug(&name), *occurrence);
        *occurrence += 1;

        result.push((
            MicrophoneDevice {
                id,
                is_default: default_name.as_deref() == Some(name.as_str()),
                name,
            },
            device,
        ));
    }

    Ok(result)
}

fn selected_microphone(app: &AppHandle) -> Result<Option<String>, String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    Ok(store
        .get(SELECTED_MICROPHONE_KEY)
        .and_then(|value| value.as_str().map(ToOwned::to_owned)))
}

fn set_snapshot(snapshot: &Arc<Mutex<MicrophoneTestSnapshot>>, value: MicrophoneTestSnapshot) {
    if let Ok(mut current) = snapshot.lock() {
        if std::mem::discriminant(&current.status) != std::mem::discriminant(&value.status) {
            crate::local_log::event("microphone", match value.status {
                MicrophoneStatus::Idle => "idle", MicrophoneStatus::Listening => "listening",
                MicrophoneStatus::Disconnected => "disconnected", MicrophoneStatus::PermissionDenied => "permission_denied",
                MicrophoneStatus::Unavailable => "unavailable", MicrophoneStatus::Error => "error",
            });
        }
        *current = value;
    }
}

fn stream_error_snapshot(
    error: cpal::StreamError,
    device_id: String,
    device_name: String,
) -> MicrophoneTestSnapshot {
    let message = error.to_string();
    let status = if permission_denied(&message) {
        MicrophoneStatus::PermissionDenied
    } else {
        MicrophoneStatus::Disconnected
    };

    MicrophoneTestSnapshot {
        status,
        level: 0.0,
        device_id: Some(device_id),
        device_name: Some(device_name),
        message: format!("The microphone stream stopped: {message}"),
        internal_format: AudioFormat::default(),
    }
}

fn build_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    device_info: &MicrophoneDevice,
    snapshot: Arc<Mutex<MicrophoneTestSnapshot>>,
) -> Result<Stream, String> {
    let mut meter = LevelMeter::new(config.sample_rate.0, config.channels);
    let error_snapshot = Arc::clone(&snapshot);
    let device_id = device_info.id.clone();
    let device_name = device_info.name.clone();
    let error_callback = move |error: cpal::StreamError| {
        crate::local_log::event("microphone", "stream_error");
        set_snapshot(
            &error_snapshot,
            stream_error_snapshot(error, device_id.clone(), device_name.clone()),
        );
    };

    let result = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            config,
            move |data: &[f32], _| meter.consume(data, |sample| *sample, &snapshot),
            error_callback,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _| {
                meter.consume(data, |sample| *sample as f32 / i16::MAX as f32, &snapshot)
            },
            error_callback,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            config,
            move |data: &[u16], _| {
                meter.consume(
                    data,
                    |sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0,
                    &snapshot,
                )
            },
            error_callback,
            None,
        ),
        format => return Err(format!("Unsupported microphone sample format: {format:?}")),
    };

    result.map_err(|error| format!("Could not create the microphone stream: {error}"))
}

struct WakeNormalizer {
    source_rate: u32,
    source_channels: usize,
    accumulator: u32,
    pending: Vec<f32>,
    sender: mpsc::Sender<AudioCommand>,
    conditioner: AudioConditioner,
}

struct AudioConditioner {
    previous_input: f32,
    previous_output: f32,
    noise_floor: f32,
    gain: f32,
}

impl Default for AudioConditioner {
    fn default() -> Self {
        Self {
            previous_input: 0.0,
            previous_output: 0.0,
            noise_floor: 0.003,
            gain: 1.0,
        }
    }
}

impl AudioConditioner {
    fn process(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            let input = sample.clamp(-1.0, 1.0);
            let filtered = input - self.previous_input + 0.995 * self.previous_output;
            self.previous_input = input;
            self.previous_output = filtered;
            *sample = filtered.clamp(-0.98, 0.98);
        }
        if samples.is_empty() {
            return;
        }
        let rms = (samples.iter().map(|sample| sample * sample).sum::<f32>()
            / samples.len() as f32)
            .sqrt();
        if rms < self.noise_floor * 2.0 + 0.004 {
            self.noise_floor = (self.noise_floor * 0.97 + rms * 0.03).clamp(0.001, 0.04);
        }
        let speech_like = rms >= (self.noise_floor * 2.2).max(0.006);
        let target = if speech_like && rms > 0.0 {
            (0.045 / rms).clamp(1.0, 2.0)
        } else {
            1.0
        };
        self.gain = self.gain * 0.94 + target * 0.06;
        for sample in samples {
            *sample = (*sample * self.gain).clamp(-0.98, 0.98);
        }
    }
}

impl WakeNormalizer {
    fn new(source_rate: u32, source_channels: u16, sender: mpsc::Sender<AudioCommand>) -> Self {
        Self {
            source_rate: source_rate.max(1),
            source_channels: usize::from(source_channels.max(1)),
            accumulator: 0,
            pending: Vec::with_capacity(WAKE_FRAME_SAMPLES * 2),
            sender,
            conditioner: AudioConditioner::default(),
        }
    }

    fn consume<T>(&mut self, samples: &[T], convert: impl Fn(&T) -> f32) {
        for frame in samples.chunks(self.source_channels) {
            let mono = frame.iter().map(&convert).sum::<f32>() / frame.len() as f32;
            self.accumulator = self.accumulator.saturating_add(INTERNAL_SAMPLE_RATE);
            while self.accumulator >= self.source_rate {
                self.accumulator -= self.source_rate;
                self.pending.push(mono.clamp(-1.0, 1.0));
            }

            if self.pending.len() >= WAKE_FRAME_SAMPLES {
                let mut samples = std::mem::replace(
                    &mut self.pending,
                    Vec::with_capacity(WAKE_FRAME_SAMPLES * 2),
                );
                self.conditioner.process(&mut samples);
                let _ = self.sender.send(AudioCommand::WakeSamples(samples));
            }
        }
    }
}

struct WakeDetector {
    spotter: KeywordSpotter,
    stream: OnlineStream,
    app: AppHandle,
    last_detection: Option<Instant>,
}

impl WakeDetector {
    fn process(&mut self, samples: &[f32]) -> Option<String> {
        self.stream
            .accept_waveform(INTERNAL_SAMPLE_RATE as i32, samples);
        let mut detected = None;

        while self.spotter.is_ready(&self.stream) {
            self.spotter.decode(&self.stream);
            if let Some(result) = self.spotter.get_result(&self.stream) {
                if !result.keyword.is_empty()
                    && self
                        .last_detection
                        .is_none_or(|previous| previous.elapsed() >= Duration::from_secs(3))
                {
                    self.last_detection = Some(Instant::now());
                    self.spotter.reset(&self.stream);
                    crate::assistant::wake_from_voice(&self.app);
                    detected = Some(match result.keyword.as_str() {
                        "NOVA" | "@NOVA" => "NOVA".to_string(),
                        _ => "Hey NOVA".to_string(),
                    });
                    break;
                }
            }
        }

        detected
    }
}

fn model_path(directory: &Path, file: &str) -> Result<String, String> {
    let path = directory.join(file);
    if !path.is_file() {
        return Err(format!(
            "Wake model asset is missing: {}",
            path.to_string_lossy()
        ));
    }
    Ok(path.to_string_lossy().into_owned())
}

fn create_keyword_spotter(
    model_directory: &Path,
    sensitivity: crate::settings::WakeSensitivity,
) -> Result<KeywordSpotter, String> {
    let mut config = KeywordSpotterConfig::default();
    config.model_config.transducer.encoder = Some(model_path(model_directory, ENCODER_FILE)?);
    config.model_config.transducer.decoder = Some(model_path(model_directory, DECODER_FILE)?);
    config.model_config.transducer.joiner = Some(model_path(model_directory, JOINER_FILE)?);
    config.model_config.tokens = Some(model_path(model_directory, TOKENS_FILE)?);
    config.model_config.model_type = Some("zipformer2".into());
    config.model_config.num_threads = 1;
    config.keywords_file = Some(model_path(model_directory, KEYWORDS_FILE)?);
    config.keywords_score = 1.5;
    config.keywords_threshold = sensitivity.threshold();
    config.max_active_paths = 4;

    KeywordSpotter::create(&config)
        .ok_or_else(|| "Could not initialize the local sherpa-onnx wake model.".to_string())
}

fn create_wake_detector(configuration: &WakeConfiguration) -> Result<WakeDetector, String> {
    let model_directory = configuration
        .model_directory
        .as_deref()
        .ok_or_else(|| "The bundled wake-word model directory is unavailable.".to_string())?;
    let spotter = create_keyword_spotter(model_directory, configuration.sensitivity)?;
    let stream = spotter.create_stream();

    Ok(WakeDetector {
        spotter,
        stream,
        app: configuration.app.clone(),
        last_detection: None,
    })
}
fn build_wake_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    sender: mpsc::Sender<AudioCommand>,
) -> Result<Stream, String> {
    let mut normalizer = WakeNormalizer::new(config.sample_rate.0, config.channels, sender.clone());
    let error_sender = sender;
    let error_callback = move |error: cpal::StreamError| {
        crate::local_log::event("microphone", "stream_error");
        let _ = error_sender.send(AudioCommand::WakeStreamError(error.to_string()));
    };

    let result = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            config,
            move |data: &[f32], _| normalizer.consume(data, |sample| *sample),
            error_callback,
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _| {
                normalizer.consume(data, |sample| *sample as f32 / i16::MAX as f32)
            },
            error_callback,
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            config,
            move |data: &[u16], _| {
                normalizer.consume(data, |sample| {
                    (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0
                })
            },
            error_callback,
            None,
        ),
        format => return Err(format!("Unsupported microphone sample format: {format:?}")),
    };

    result.map_err(|error| format!("Could not create the wake microphone stream: {error}"))
}

fn start_wake_stream(
    configuration: &WakeConfiguration,
    sender: mpsc::Sender<AudioCommand>,
) -> Result<(Stream, WakeDetector, String), String> {
    let devices = enumerate_devices()?;
    let selected = match configuration.device_id.as_deref() {
        Some(id) => devices
            .iter()
            .find(|(details, _)| details.id == id)
            .ok_or_else(|| "The selected wake microphone is disconnected.".to_string())?,
        None => devices
            .iter()
            .find(|(details, _)| details.is_default)
            .or_else(|| devices.first())
            .ok_or_else(|| "No microphone input devices are available.".to_string())?,
    };
    let (details, device) = selected;
    let supported = device
        .default_input_config()
        .map_err(|error| format!("Could not read the wake microphone format: {error}"))?;
    let config: StreamConfig = supported.clone().into();
    let detector = create_wake_detector(configuration)?;
    let stream = build_wake_stream(device, &config, supported.sample_format(), sender)?;

    stream
        .play()
        .map_err(|error| format!("Could not start wake detection: {error}"))?;

    Ok((stream, detector, details.name.clone()))
}

fn wake_error(message: String) -> WakeEngineSnapshot {
    WakeEngineSnapshot {
        engine: WAKE_ENGINE_NAME,
        phrase: WAKE_PHRASE,
        status: WakeEngineStatus::Error,
        message,
        device_name: None,
        input_level: 0.0,
        noise_floor: 0.0,
        sensitivity: crate::settings::WakeSensitivity::default(),
        threshold: crate::settings::WakeSensitivity::default().threshold(),
        last_detected_phrase: None,
        last_wake_confidence: None,
        last_detection_millis: None,
    }
}

fn apply_wake_configuration(
    configuration: &WakeConfiguration,
    sender: mpsc::Sender<AudioCommand>,
) -> (Option<Stream>, Option<WakeDetector>, WakeEngineSnapshot) {
    if !configuration.enabled {
        return (None, None, WakeEngineSnapshot::disabled());
    }

    if configuration.paused {
        return (
            None,
            None,
            WakeEngineSnapshot {
                engine: WAKE_ENGINE_NAME,
                phrase: WAKE_PHRASE,
                status: WakeEngineStatus::Paused,
                message: "Wake detection is paused.".into(),
                device_name: None,
                input_level: 0.0,
                noise_floor: 0.0,
                sensitivity: configuration.sensitivity,
                threshold: configuration.sensitivity.threshold(),
                last_detected_phrase: None,
                last_wake_confidence: None,
                last_detection_millis: None,
            },
        );
    }

    match start_wake_stream(configuration, sender) {
        Ok((stream, detector, device_name)) => (
            Some(stream),
            Some(detector),
            WakeEngineSnapshot {
                engine: WAKE_ENGINE_NAME,
                phrase: WAKE_PHRASE,
                status: WakeEngineStatus::Listening,
                message: "Listening locally for “NOVA” or “Hey NOVA”.".into(),
                device_name: Some(device_name),
                input_level: 0.0,
                noise_floor: 0.0,
                sensitivity: configuration.sensitivity,
                threshold: configuration.sensitivity.threshold(),
                last_detected_phrase: None,
                last_wake_confidence: None,
                last_detection_millis: None,
            },
        ),
        Err(error) => (None, None, wake_error(error)),
    }
}

fn set_wake_snapshot(snapshot: &Arc<Mutex<WakeEngineSnapshot>>, value: WakeEngineSnapshot) {
    if let Ok(mut current) = snapshot.lock() {
        if std::mem::discriminant(&current.status) != std::mem::discriminant(&value.status) {
            crate::local_log::event("wake", match value.status {
                WakeEngineStatus::Listening => "listening", WakeEngineStatus::Paused => "paused",
                WakeEngineStatus::Disabled => "disabled", WakeEngineStatus::Error => "error",
            });
        }
        *current = value;
    }
}

fn wake_model_directory(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = crate::runtime_paths::resource(app, WAKE_MODEL_DIRECTORY)?;
    if directory.is_dir() { Ok(directory) } else {
        Err("The bundled wake-word model is missing. Reinstall NOVA.".into())
    }
}
#[tauri::command]
pub fn list_microphone_devices(app: AppHandle) -> Result<MicrophoneDeviceList, String> {
    let devices = enumerate_devices()?
        .into_iter()
        .map(|(details, _)| details)
        .collect();

    Ok(MicrophoneDeviceList {
        devices,
        selected_device_id: selected_microphone(&app)?,
        internal_format: AudioFormat::default(),
    })
}

#[tauri::command]
pub fn set_selected_microphone(
    app: AppHandle,
    audio_service: State<'_, AudioService>,
    arguments: SelectMicrophoneArguments,
) -> Result<String, String> {
    if !enumerate_devices()?
        .iter()
        .any(|(details, _)| details.id == arguments.device_id)
    {
        return Err("The selected microphone is no longer available.".into());
    }

    let store = app
        .store(SETTINGS_FILE)
        .map_err(|error| format!("Could not open NOVA settings: {error}"))?;
    store.set(
        SELECTED_MICROPHONE_KEY,
        Value::String(arguments.device_id.clone()),
    );
    store
        .save()
        .map_err(|error| format!("Could not save the selected microphone: {error}"))?;
    synchronize_wake_engine(&app, &audio_service)?;

    Ok(arguments.device_id)
}

fn start_stream(
    requested: Option<String>,
    snapshot: &Arc<Mutex<MicrophoneTestSnapshot>>,
) -> Result<(Stream, MicrophoneTestSnapshot), String> {
    let devices = enumerate_devices()?;
    let selected = match requested.as_deref() {
        Some(id) => devices
            .iter()
            .find(|(details, _)| details.id == id)
            .ok_or_else(|| "The selected microphone is no longer available.".to_string())?,
        None => devices
            .iter()
            .find(|(details, _)| details.is_default)
            .or_else(|| devices.first())
            .ok_or_else(|| "No microphone input devices are available.".to_string())?,
    };
    let (details, device) = selected;
    let supported = device
        .default_input_config()
        .map_err(|error| format!("Could not read the microphone format: {error}"))?;
    let config: StreamConfig = supported.clone().into();

    let listening = MicrophoneTestSnapshot {
        status: MicrophoneStatus::Listening,
        level: 0.0,
        device_id: Some(details.id.clone()),
        device_name: Some(details.name.clone()),
        message: "Listening locally. Test audio is not stored.".into(),
        internal_format: AudioFormat::default(),
    };
    set_snapshot(snapshot, listening.clone());

    let stream = match build_stream(
        device,
        &config,
        supported.sample_format(),
        details,
        Arc::clone(snapshot),
    ) {
        Ok(stream) => stream,
        Err(error) => {
            let status = if permission_denied(&error) {
                MicrophoneStatus::PermissionDenied
            } else {
                MicrophoneStatus::Error
            };
            set_snapshot(
                snapshot,
                MicrophoneTestSnapshot {
                    status,
                    level: 0.0,
                    device_id: Some(details.id.clone()),
                    device_name: Some(details.name.clone()),
                    message: error.clone(),
                    internal_format: AudioFormat::default(),
                },
            );
            return Err(error);
        }
    };

    stream.play().map_err(|error| {
        let message = format!("Could not start the microphone stream: {error}");
        let status = if permission_denied(&message) {
            MicrophoneStatus::PermissionDenied
        } else {
            MicrophoneStatus::Error
        };
        set_snapshot(
            snapshot,
            MicrophoneTestSnapshot {
                status,
                level: 0.0,
                device_id: Some(details.id.clone()),
                device_name: Some(details.name.clone()),
                message: message.clone(),
                internal_format: AudioFormat::default(),
            },
        );
        message
    })?;

    Ok((stream, listening))
}

fn restore_wake(
    desired_wake: Option<&WakeConfiguration>,
    sender: &mpsc::Sender<AudioCommand>,
    active_stream: &mut Option<Stream>,
    detector: &mut Option<WakeDetector>,
    wake_snapshot: &Arc<Mutex<WakeEngineSnapshot>>,
) {
    let previous_detection = wake_snapshot.lock().ok().map(|state| {
        (
            state.last_detection_millis,
            state.last_detected_phrase.clone(),
            state.last_wake_confidence,
        )
    });
    *active_stream = None;
    *detector = None;
    if let Some(configuration) = desired_wake {
        let (stream, next_detector, mut state) =
            apply_wake_configuration(configuration, sender.clone());
        if let Some((timestamp, phrase, confidence)) = previous_detection {
            state.last_detection_millis = timestamp;
            state.last_detected_phrase = phrase;
            state.last_wake_confidence = confidence;
        }
        *active_stream = stream;
        *detector = next_detector;
        set_wake_snapshot(wake_snapshot, state);
    }
}

fn normalized_input_level(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares = samples
        .iter()
        .map(|sample| f64::from(sample.clamp(-1.0, 1.0).powi(2)))
        .sum::<f64>();
    ((sum_squares / samples.len() as f64).sqrt() as f32 * 3.0).clamp(0.0, 1.0)
}
fn audio_worker(
    commands: mpsc::Receiver<AudioCommand>,
    sender: mpsc::Sender<AudioCommand>,
    test_snapshot: Arc<Mutex<MicrophoneTestSnapshot>>,
    wake_snapshot: Arc<Mutex<WakeEngineSnapshot>>,
    transcriber: SpeechTranscriber,
) {
    let mut active_stream: Option<Stream> = None;
    let mut detector: Option<WakeDetector> = None;
    let mut command_capture: Option<CommandCapture> = None;
    let mut followup_capture = false;
    let mut desired_wake: Option<WakeConfiguration> = None;
    let mut testing = false;
    let mut transcribing = false;
    let mut generation = 0u64;

    while let Ok(command) = commands.recv() {
        match command {
            AudioCommand::ConfigureWake {
                configuration,
                reply,
            } => {
                generation = generation.wrapping_add(1);
                command_capture = None;
                followup_capture = false;
                transcribing = false;
                desired_wake = Some(configuration.clone());
                let state = if testing && configuration.enabled {
                    WakeEngineSnapshot {
                        engine: WAKE_ENGINE_NAME,
                        phrase: WAKE_PHRASE,
                        status: WakeEngineStatus::Paused,
                        message: "Wake detection is temporarily paused during the microphone test."
                            .into(),
                        device_name: None,
                        input_level: 0.0,
                        noise_floor: 0.0,
                        sensitivity: configuration.sensitivity,
                        threshold: configuration.sensitivity.threshold(),
                        last_detected_phrase: None,
                        last_wake_confidence: None,
                        last_detection_millis: None,
                    }
                } 
                else {
                    if !configuration.enabled {
                        testing = false;
                        set_snapshot(&test_snapshot, MicrophoneTestSnapshot::idle());
                    }
                    drop(active_stream.take());
                    drop(detector.take());
                    let (stream, next_detector, state) =
                        apply_wake_configuration(&configuration, sender.clone());
                    active_stream = stream;
                    detector = next_detector;
                    state
                };
                set_wake_snapshot(&wake_snapshot, state.clone());
                let _ = reply.send(state);
            }
            AudioCommand::WakeSamples(samples) => {
                let measured_level = normalized_input_level(&samples);
                if let Ok(mut state) = wake_snapshot.lock() {
                    if matches!(state.status, WakeEngineStatus::Listening) {
                        state.input_level = state.input_level * 0.75 + measured_level * 0.25;
                        let rms = measured_level / 3.0;
                        if rms < state.noise_floor * 2.0 + 0.004 || state.noise_floor == 0.0 {
                            state.noise_floor = state.noise_floor * 0.95 + rms * 0.05;
                        }
                    }
                }
                if testing || transcribing {
                    continue;
                }

                if let Some(capture) = command_capture.as_mut() {
                    let outcome = capture.process(&samples);
                    if let Some(configuration) = desired_wake.as_ref() {
                        let _ = configuration.app.emit_to(
                            "assistant",
                            "assistant-voice-level",
                            capture.voice_level(),
                        );
                    }
                    match outcome {
                        CaptureOutcome::Continue => {}
                        CaptureOutcome::Complete(audio) => {
                            command_capture = None;
                            followup_capture = false;
                            active_stream = None;
                            detector = None;
                            transcribing = true;
                            if let Some(configuration) = desired_wake.as_ref() {
                                crate::assistant::update_voice_state(
                                    &configuration.app,
                                    "transcribing",
                                    None,
                                    None,
                                );
                            }
                            let model_path = desired_wake
                                .as_ref()
                                .map(|configuration| configuration.speech_model_path.clone())
                                .expect("wake configuration exists");
                            let current_generation = generation;
                            match transcriber.transcribe(model_path, audio) {
                                Ok(receiver) => {
                                    let completion_sender = sender.clone();
                                    if let Err(error) = std::thread::Builder::new()
                                        .name("nova-stt-completion".into())
                                        .spawn(move || {
                                            let result = receiver.recv().unwrap_or_else(|_| {
                                                Err(SpeechError::new(
                                                    SpeechErrorCode::EngineFailure,
                                                    "The local speech worker did not return a result.",
                                                ))
                                            });
                                            let _ = completion_sender.send(
                                                AudioCommand::TranscriptionFinished {
                                                    generation: current_generation,
                                                    result,
                                                },
                                            );
                                        })
                                    {
                                        let _ = sender.send(AudioCommand::TranscriptionFinished {
                                            generation: current_generation,
                                            result: Err(SpeechError::new(
                                                SpeechErrorCode::EngineFailure,
                                                format!(
                                                    "Could not wait for local transcription: {error}"
                                                ),
                                            )),
                                        });
                                    }
                                }
                                Err(error) => {
                                    let _ = sender.send(AudioCommand::TranscriptionFinished {
                                        generation: current_generation,
                                        result: Err(SpeechError::new(
                                            SpeechErrorCode::EngineFailure,
                                            error,
                                        )),
                                    });
                                }
                            }
                        }
                        CaptureOutcome::Error(error) => {
                            command_capture = None;
                            let was_followup = followup_capture;
                            followup_capture = false;
                            if let Some(configuration) = desired_wake.as_ref() {
                                if was_followup && matches!(error.code, SpeechErrorCode::NoSpeech) {
                                    crate::assistant::update_voice_state(
                                        &configuration.app,
                                        "idle",
                                        None,
                                        None,
                                    );
                                } else {
                                    crate::assistant::update_voice_state(
                                        &configuration.app,
                                        "error",
                                        None,
                                        Some(error.message),
                                    );
                                }
                            }
                            restore_wake(
                                desired_wake.as_ref(),
                                &sender,
                                &mut active_stream,
                                &mut detector,
                                &wake_snapshot,
                            );
                        }
                    }
                    continue;
                }

                if let Some(active_detector) = detector.as_mut() {
                    if let Some(phrase) = active_detector.process(&samples) {
                        generation = generation.wrapping_add(1);
                        command_capture = match desired_wake.as_ref().map(|config| CommandCapture::new(&config.app)) {
                            Some(Ok(capture)) => Some(capture),
                            Some(Err(error)) => {
                                if let Some(config) = desired_wake.as_ref() {
                                    crate::assistant::update_voice_state(&config.app, "error", None, Some(error));
                                }
                                None
                            }
                            None => None,
                        };
                        followup_capture = false;
                        let last_detection_millis = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .ok()
                            .map(|duration| duration.as_millis());
                        if let Ok(mut state) = wake_snapshot.lock() {
                            state.message =
                                "Wake phrase detected; recording the spoken command locally."
                                    .into();
                            state.last_detection_millis = last_detection_millis;
                            state.last_detected_phrase = Some(phrase);
                            // This KWS API exposes phrase/token timing but no
                            // calibrated posterior, so confidence remains null.
                            state.last_wake_confidence = None;
                        }
                    }
                }
            }
            AudioCommand::WakeStreamError(error) => {
                if !testing {
                    let was_capturing = command_capture.take().is_some();
                    followup_capture = false;
                    active_stream = None;
                    detector = None;
                    set_wake_snapshot(
                        &wake_snapshot,
                        wake_error(format!("Wake microphone disconnected: {error}")),
                    );
                    if was_capturing {
                        if let Some(configuration) = desired_wake.as_ref() {
                            crate::assistant::update_voice_state(
                                &configuration.app,
                                "error",
                                None,
                                Some(format!("The microphone disconnected: {error}")),
                            );
                        }
                    }
                }
            }
            AudioCommand::TranscriptionFinished {
                generation: completed_generation,
                result,
            } => {
                if completed_generation != generation || !transcribing {
                    continue;
                }
                transcribing = false;
                if let Some(configuration) = desired_wake.as_ref() {
                    match result {
                        Ok(transcription) => {
                            eprintln!(
                                "Local speech transcription completed (confidence {:.2}).",
                                transcription.confidence
                            );
                            let command_text = transcription.text;
                            crate::assistant::update_voice_state(
                                &configuration.app,
                                "processing",
                                Some(command_text.clone()),
                                None,
                            );
                            crate::language_model::process_voice_command(
                                configuration.app.clone(),
                                command_text,
                            );
                        }
                        Err(error) if matches!(error.code, SpeechErrorCode::NoSpeech | SpeechErrorCode::LowConfidence)
                            && crate::assistant::is_assistant_visible(&configuration.app) => {
                            // Ambiguous sound never reaches the tool router. Resume listening.
                            match CommandCapture::new(&configuration.app) {
                                Ok(capture) => {
                                    command_capture = Some(capture);
                                    followup_capture = true;
                                    crate::assistant::update_voice_state(&configuration.app, "listening", None, Some("I didn't catch that. Please repeat a short command.".into()));
                                }
                                Err(message) => crate::assistant::update_voice_state(&configuration.app, "error", None, Some(message)),
                            }
                        }
                        Err(error) => {
                            eprintln!(
                                "Local speech transcription failed ({:?}): {}",
                                error.code, error.message
                            );
                            crate::assistant::update_voice_state(
                                &configuration.app,
                                "error",
                                None,
                                Some(error.message),
                            );
                        }
                    }
                }
                restore_wake(
                    desired_wake.as_ref(),
                    &sender,
                    &mut active_stream,
                    &mut detector,
                    &wake_snapshot,
                );
            }
            AudioCommand::BeginFollowup { reply, speech_generation } => {
                let result = match desired_wake.as_ref() {
                    Some(configuration) if speech_generation.is_some_and(|generation| !configuration.app.state::<crate::tts::TtsService>().current(generation)) => {
                        Err("The voice reply was interrupted.".into())
                    }
                    Some(configuration) if !configuration.enabled => {
                        Err("NOVA is disabled.".to_string())
                    }
                    Some(configuration) if configuration.paused => {
                        Err("The assistant is paused.".to_string())
                    }
                    Some(configuration)
                        if !crate::assistant::is_assistant_visible(&configuration.app) =>
                    {
                        Err("The floating assistant is hidden.".to_string())
                    }
                    None => Err("The wake engine is not configured.".to_string()),
                    Some(_) if testing => Err("A microphone test is already running.".to_string()),
                    Some(_) if transcribing => {
                        Err("A transcription is already running.".to_string())
                    }
                    Some(configuration) => {
                        if active_stream.is_none() {
                            restore_wake(desired_wake.as_ref(), &sender, &mut active_stream, &mut detector, &wake_snapshot);
                        }
                        if active_stream.is_none() {
                            Err("The microphone is unavailable. Check Voice & wake word.".into())
                        } else {
                            match CommandCapture::new(&configuration.app) {
                                Ok(capture) => {
                                    generation = generation.wrapping_add(1);
                                    command_capture = Some(capture);
                                    followup_capture = true;
                                    if let Ok(mut state) = wake_snapshot.lock() {
                                        state.message = "Listening for speech.".into();
                                    }
                                    Ok(())
                                }
                                Err(error) => Err(error),
                            }
                        }
                    }
                };
                let _ = reply.send(result);
            }
            AudioCommand::CancelVoiceSession { reply } => {
                generation = generation.wrapping_add(1);
                command_capture = None;
                followup_capture = false;
                transcribing = false;
                if !testing {
                    restore_wake(
                        desired_wake.as_ref(),
                        &sender,
                        &mut active_stream,
                        &mut detector,
                        &wake_snapshot,
                    );
                }
                let _ = reply.send(());
            }
            AudioCommand::Start { device_id, reply } => {
                generation = generation.wrapping_add(1);
                active_stream = None;
                detector = None;
                command_capture = None;
                transcribing = false;
                testing = true;
                set_wake_snapshot(
                    &wake_snapshot,
                    WakeEngineSnapshot {
                        engine: WAKE_ENGINE_NAME,
                        phrase: WAKE_PHRASE,
                        status: WakeEngineStatus::Paused,
                        message: "Wake detection is temporarily paused during the microphone test."
                            .into(),
                        device_name: None,
                        input_level: 0.0,
                        noise_floor: 0.0,
                        sensitivity: desired_wake
                            .as_ref()
                            .map(|configuration| configuration.sensitivity)
                            .unwrap_or_default(),
                        threshold: desired_wake
                            .as_ref()
                            .map(|configuration| configuration.sensitivity.threshold())
                            .unwrap_or_else(|| {
                                crate::settings::WakeSensitivity::default().threshold()
                            }),
                        last_detected_phrase: None,
                        last_wake_confidence: None,
                        last_detection_millis: None,
                    },
                );

                let result = start_stream(device_id, &test_snapshot)
                    .map(|(stream, state)| {
                        active_stream = Some(stream);
                        state
                    })
                    .map_err(|error| {
                        testing = false;
                        let status = if permission_denied(&error) {
                            MicrophoneStatus::PermissionDenied
                        } else if error.contains("no longer available") {
                            MicrophoneStatus::Disconnected
                        } else if error.contains("No microphone input devices") {
                            MicrophoneStatus::Unavailable
                        } else {
                            MicrophoneStatus::Error
                        };
                        set_snapshot(
                            &test_snapshot,
                            MicrophoneTestSnapshot {
                                status,
                                level: 0.0,
                                device_id: None,
                                device_name: None,
                                message: error.clone(),
                                internal_format: AudioFormat::default(),
                            },
                        );
                        restore_wake(
                            desired_wake.as_ref(),
                            &sender,
                            &mut active_stream,
                            &mut detector,
                            &wake_snapshot,
                        );
                        error
                    });
                let _ = reply.send(result);
            }
            AudioCommand::Stop { reply } => {
                generation = generation.wrapping_add(1);
                active_stream = None;
                detector = None;
                command_capture = None;
                transcribing = false;
                testing = false;
                let stopped = MicrophoneTestSnapshot::idle();
                set_snapshot(&test_snapshot, stopped.clone());
                restore_wake(
                    desired_wake.as_ref(),
                    &sender,
                    &mut active_stream,
                    &mut detector,
                    &wake_snapshot,
                );
                let _ = reply.send(stopped);
            }
        }
    }

    drop(detector);
    drop(active_stream);
}
pub fn synchronize_wake_engine(
    app: &AppHandle,
    service: &AudioService,
) -> Result<WakeEngineSnapshot, String> {
    let enabled = crate::settings::nova_enabled(app)?;
    let paused = crate::settings::assistant_paused(app)?;
    let configuration = WakeConfiguration {
        app: app.clone(),
        enabled,
        paused,
        sensitivity: crate::settings::wake_sensitivity(app)?,
        device_id: selected_microphone(app)?,
        model_directory: if enabled && !paused {
            Some(wake_model_directory(app)?)
        } else {
            None
        },
        // OFF and pause must release capture even with a stale or invalid model override.
        speech_model_path: if enabled && !paused { crate::speech::effective_model_path(app)? } else { PathBuf::new() },
    };
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_sender(service)?
        .send(AudioCommand::ConfigureWake {
            configuration,
            reply: reply_tx,
        })
        .map_err(|_| "The local audio service stopped unexpectedly.".to_string())?;
    Ok(reply_rx
        .recv()
        .map_err(|_| "The local wake engine did not return a status.".to_string())?)
}

pub fn begin_followup_capture(
    service: &AudioService,
    speech_generation: Option<u64>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_sender(service)?
        .send(AudioCommand::BeginFollowup {
            reply: reply_tx,
            speech_generation,
        })
        .map_err(|_| "The local audio service stopped unexpectedly.".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "The local audio service did not start follow-up listening.".to_string())?
}

pub fn cancel_voice_session(service: &AudioService) -> Result<(), String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_sender(service)?
        .send(AudioCommand::CancelVoiceSession { reply: reply_tx })
        .map_err(|_| "The local audio service stopped unexpectedly.".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "The local audio service did not cancel the voice session.".to_string())
}

#[tauri::command]
pub fn get_wake_engine_status(service: State<'_, AudioService>) -> WakeEngineSnapshot {
    service
        .wake_snapshot
        .lock()
        .map(|snapshot| snapshot.clone())
        .unwrap_or_else(|_| wake_error("The wake-engine status is unavailable.".into()))
}
fn audio_sender(service: &AudioService) -> Result<mpsc::Sender<AudioCommand>, String> {
    service
        .command_tx
        .lock()
        .map_err(|_| "The local audio service is unavailable.".to_string())?
        .clone()
        .ok_or_else(|| "The local audio service could not be started.".to_string())
}

#[tauri::command]
pub fn start_microphone_test(
    app: AppHandle,
    service: State<'_, AudioService>,
    device_id: Option<String>,
) -> Result<MicrophoneTestSnapshot, String> {
    let requested = device_id.or(selected_microphone(&app)?);
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_sender(&service)?
        .send(AudioCommand::Start {
            device_id: requested,
            reply: reply_tx,
        })
        .map_err(|_| "The local audio service stopped unexpectedly.".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "The local audio service did not return a result.".to_string())?
}

#[tauri::command]
pub fn get_microphone_test_state(service: State<'_, AudioService>) -> MicrophoneTestSnapshot {
    service
        .snapshot
        .lock()
        .map(|snapshot| snapshot.clone())
        .unwrap_or_else(|_| MicrophoneTestSnapshot {
            status: MicrophoneStatus::Unavailable,
            level: 0.0,
            device_id: None,
            device_name: None,
            message: "The microphone test state is unavailable.".into(),
            internal_format: AudioFormat::default(),
        })
}

#[tauri::command]
pub fn stop_microphone_test(
    service: State<'_, AudioService>,
) -> Result<MicrophoneTestSnapshot, String> {
    let (reply_tx, reply_rx) = mpsc::channel();
    audio_sender(&service)?
        .send(AudioCommand::Stop { reply: reply_tx })
        .map_err(|_| "The local audio service stopped unexpectedly.".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "The local audio service did not return a result.".to_string())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_slug_is_stable_and_safe() {
        assert_eq!(slug("Microphone (USB Audio)"), "microphone-usb-audio");
    }

    #[test]
    fn permission_errors_are_classified() {
        assert!(permission_denied("Access denied (0x80070005)"));
        assert!(!permission_denied("Device is not available"));
    }

    #[test]
    #[ignore = "requires an available local microphone"]
    fn live_microphone_capture_smoke_test() {
        let snapshot = Arc::new(Mutex::new(MicrophoneTestSnapshot::idle()));
        let (stream, state) =
            start_stream(None, &snapshot).expect("an available microphone should start");
        assert!(matches!(state.status, MicrophoneStatus::Listening));
        std::thread::sleep(std::time::Duration::from_millis(350));
        assert!(matches!(
            snapshot.lock().unwrap().status,
            MicrophoneStatus::Listening
        ));
        drop(stream);
    }

    #[test]
    #[ignore = "requires NOVA_WAKE_TEST_WAV with a spoken Hey Nova sample"]
    fn bundled_model_detects_hey_nova_audio() {
        let path = std::env::var("NOVA_WAKE_TEST_WAV")
            .expect("NOVA_WAKE_TEST_WAV should point to a local WAV");
        let wave = sherpa_onnx::Wave::read(&path).expect("wake test WAV should be readable");
        let model_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(WAKE_MODEL_DIRECTORY);
        let spotter =
            create_keyword_spotter(&model_directory, crate::settings::WakeSensitivity::High)
                .expect("bundled wake model should initialize");
        let stream = spotter.create_stream();
        stream.accept_waveform(wave.sample_rate(), wave.samples());
        stream.accept_waveform(
            wave.sample_rate(),
            &vec![0.0; (wave.sample_rate() / 2) as usize],
        );
        stream.input_finished();

        let mut detected = false;
        while spotter.is_ready(&stream) {
            spotter.decode(&stream);
            if spotter
                .get_result(&stream)
                .is_some_and(|result| !result.keyword.is_empty())
            {
                detected = true;
                break;
            }
        }
        assert!(detected, "the bundled model did not detect Hey NOVA");
    }

    #[test]
    fn level_meter_produces_a_bounded_level() {
        let snapshot = Arc::new(Mutex::new(MicrophoneTestSnapshot::idle()));
        let mut meter = LevelMeter::new(INTERNAL_SAMPLE_RATE, 2);
        meter.consume(
            &vec![0.5; LEVEL_WINDOW_SAMPLES * 2],
            |sample| *sample,
            &snapshot,
        );
        let level = snapshot.lock().unwrap().level;
        assert!((0.0..=1.0).contains(&level));
        assert!(level > 0.0);
    }

    #[test]
    fn bundled_keywords_include_both_supported_wake_phrases() {
        let keywords = std::fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(WAKE_MODEL_DIRECTORY)
                .join(KEYWORDS_FILE),
        )
        .expect("read bundled wake keywords");
        assert!(keywords.contains("@HEY_NOVA"));
        assert!(keywords.lines().any(|line| line.ends_with("@NOVA")));
    }
}
