use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use serde::Serialize;
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineWhisperModelConfig, SileroVadModelConfig, VadModelConfig, VoiceActivityDetector};
use tauri::{AppHandle, Manager};

pub const SPEECH_ENGINE_NAME: &str = "Whisper Tiny English (sherpa-onnx 1.13.7)";
pub const DEFAULT_MODEL_NAME: &str = "Whisper Tiny English INT8";
const SPEECH_RESOURCE_DIRECTORY: &str = "resources/speech";
const DEFAULT_MODEL_DIRECTORY: &str = "sherpa-onnx-whisper-tiny.en";
const ENCODER_FILE: &str = "tiny.en-encoder.int8.onnx";
const DECODER_FILE: &str = "tiny.en-decoder.int8.onnx";
const TOKENS_FILE: &str = "tiny.en-tokens.txt";
const MIN_TRANSCRIPTION_SAMPLES: usize = 4_800;
const MAX_TRANSCRIPTION_SAMPLES: usize = 160_000;
#[cfg(test)]
const CAPTURE_FRAME_SAMPLES: usize = 480;
const MAX_COMMAND_SAMPLES: usize = 10 * 16_000;
const END_SILENCE_SAMPLES: usize = 14_400;
const MIN_VOICED_SAMPLES: usize = 4_800;
const PRE_ROLL_SAMPLES: usize = 3_200;
const MIN_SPEECH_RMS_THRESHOLD: f32 = 0.008;
const NOISE_MULTIPLIER: f32 = 2.4;
const SPEECH_START_FRAMES: usize = 2;

#[derive(Debug)]
pub enum CaptureOutcome {
    Continue,
    Complete(Vec<f32>),
    Error(SpeechError),
}

pub struct CommandCapture {
    vad: Option<VoiceActivityDetector>,
    total_samples: usize,
    speech_started: bool,
    too_long: bool,
    voice_level: f32,
    consecutive_voice_frames: usize,
    voiced_samples: usize,
    silence_samples: usize,
    pre_roll: VecDeque<f32>,
    captured: Vec<f32>,
    noise_floor: f32,
    previous_input: f32,
    previous_output: f32,
    gain: f32,
}

impl Default for CommandCapture {
    fn default() -> Self {
        Self {
            vad: None,
            total_samples: 0,
            speech_started: false,
            too_long: false,
            voice_level: 0.0,
            consecutive_voice_frames: 0,
            voiced_samples: 0,
            silence_samples: 0,
            pre_roll: VecDeque::with_capacity(PRE_ROLL_SAMPLES),
            captured: Vec::with_capacity(MAX_COMMAND_SAMPLES),
            noise_floor: 0.003,
            previous_input: 0.0,
            previous_output: 0.0,
            gain: 1.0,
        }
    }
}

impl CommandCapture {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let resource = app.path().resource_dir().map_err(|error| error.to_string())?
            .join("resources/vad/silero_vad.onnx");
        let model = if resource.is_file() { resource } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/vad/silero_vad.onnx")
        };
        Self::with_vad(&model)
    }

    fn with_vad(model: &Path) -> Result<Self, String> {
        if !model.is_file() {
            return Err("The speech activity model is missing. Reinstall NOVA to enable listening.".into());
        }
        let config = VadModelConfig {
            silero_vad: SileroVadModelConfig {
                model: Some(model.to_string_lossy().into_owned()),
                threshold: 0.65,
                min_silence_duration: 0.12,
                min_speech_duration: 0.15,
                window_size: 512,
                max_speech_duration: 60.0,
            },
            sample_rate: 16_000,
            num_threads: 1,
            provider: Some("cpu".into()),
            ..Default::default()
        };
        let vad = VoiceActivityDetector::create(&config, 12.0)
            .ok_or("Could not load the local speech activity detector.")?;
        Ok(Self { vad: Some(vad), ..Self::default() })
    }

    fn reset_utterance(&mut self) {
        self.speech_started = false;
        self.too_long = false;
        self.consecutive_voice_frames = 0;
        self.voiced_samples = 0;
        self.silence_samples = 0;
        self.total_samples = 0;
        self.voice_level = 0.0;
        self.captured.clear();
        self.pre_roll.clear();
        if let Some(vad) = &self.vad { vad.reset(); }
    }

    pub fn voice_level(&self) -> f32 {
        self.voice_level
    }

    pub fn process(&mut self, samples: &[f32]) -> CaptureOutcome {
        self.voice_level = 0.0;
        if samples.is_empty() {
            return CaptureOutcome::Continue;
        }
        let mut conditioned = Vec::with_capacity(samples.len());
        for sample in samples {
            // Remove microphone DC bias/sub-audible rumble before VAD and STT.
            let input = sample.clamp(-1.0, 1.0);
            let filtered = input - self.previous_input + 0.995 * self.previous_output;
            self.previous_input = input;
            self.previous_output = filtered;
            conditioned.push(filtered.clamp(-0.98, 0.98));
        }
        let raw_rms = (conditioned
            .iter()
            .map(|sample| sample.clamp(-1.0, 1.0).powi(2))
            .sum::<f32>()
            / conditioned.len() as f32)
            .sqrt();
        if !self.speech_started && raw_rms < self.noise_floor * 2.0 + 0.004 {
            self.noise_floor = (self.noise_floor * 0.94 + raw_rms * 0.06).clamp(0.001, 0.04);
        }
        let speech_threshold = MIN_SPEECH_RMS_THRESHOLD.max(self.noise_floor * NOISE_MULTIPLIER);
        // Production captures always have the neural VAD. Energy alone is used
        // only by the deterministic endpoint unit tests.
        let speech_detected = if let Some(vad) = &self.vad {
            vad.accept_waveform(&conditioned);
            let detected = vad.detected();
            while !vad.is_empty() { vad.pop(); }
            detected
        } else {
            cfg!(test)
        };
        let voiced = speech_detected && raw_rms >= speech_threshold;
        self.voice_level = if voiced { (raw_rms * 8.0).clamp(0.0, 1.0) } else { 0.0 };
        // Conservative speech-only AGC: never amplify by more than 2.5x and
        // never boost frames classified as steady background noise.
        let target_gain = if voiced && raw_rms > 0.0 {
            (0.055 / raw_rms).clamp(1.0, 2.5)
        } else {
            1.0
        };
        self.gain = self.gain * 0.9 + target_gain * 0.1;
        for sample in &mut conditioned {
            *sample = (*sample * self.gain).clamp(-0.98, 0.98);
        }
        self.total_samples = self.total_samples.saturating_add(conditioned.len());

        if !self.speech_started {
            for sample in &conditioned {
                if self.pre_roll.len() == PRE_ROLL_SAMPLES {
                    self.pre_roll.pop_front();
                }
                self.pre_roll.push_back(*sample);
            }
            self.consecutive_voice_frames = if voiced {
                self.consecutive_voice_frames + 1
            } else {
                0
            };
            if self.consecutive_voice_frames >= SPEECH_START_FRAMES {
                self.speech_started = true;
                self.captured.extend(self.pre_roll.drain(..));
                self.voiced_samples = self.voiced_samples.saturating_add(conditioned.len());
            }
            return CaptureOutcome::Continue;
        }

        if self.captured.len() + conditioned.len() > MAX_COMMAND_SAMPLES {
            self.too_long = true;
        }
        let available = MAX_COMMAND_SAMPLES.saturating_sub(self.captured.len());
        self.captured.extend(conditioned.into_iter().take(available));
        if voiced {
            self.voiced_samples = self.voiced_samples.saturating_add(samples.len());
            self.silence_samples = 0;
        } else {
            self.silence_samples = self.silence_samples.saturating_add(samples.len());
        }

        if self.silence_samples >= END_SILENCE_SAMPLES {
            if self.too_long {
                return CaptureOutcome::Error(SpeechError::new(
                    SpeechErrorCode::Timeout,
                    "That command was too long. Please use a command under ten seconds.",
                ));
            }
            if self.voiced_samples < MIN_VOICED_SAMPLES {
                self.reset_utterance();
                return CaptureOutcome::Continue;
            }
            let keep_silence = PRE_ROLL_SAMPLES.min(self.silence_samples);
            let trim = self.silence_samples.saturating_sub(keep_silence);
            self.captured
                .truncate(self.captured.len().saturating_sub(trim));
            return CaptureOutcome::Complete(std::mem::take(&mut self.captured));
        }

        CaptureOutcome::Continue
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechEngineInfo {
    pub engine: &'static str,
    pub model: &'static str,
    pub model_path: String,
    pub model_available: bool,
    pub language: &'static str,
    pub sample_rate: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SpeechErrorCode {
    NoSpeech,
    LowConfidence,
    ModelMissing,
    Timeout,
    EngineFailure,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechError {
    pub code: SpeechErrorCode,
    pub message: String,
}

impl SpeechError {
    pub fn new(code: SpeechErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpeechTranscription {
    pub text: String,
    pub confidence: f32,
}

struct TranscriptionJob {
    model_path: PathBuf,
    samples: Vec<f32>,
    reply: mpsc::Sender<Result<SpeechTranscription, SpeechError>>,
}

#[derive(Clone)]
pub struct SpeechTranscriber {
    sender: Option<mpsc::Sender<TranscriptionJob>>,
}

impl Default for SpeechTranscriber {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        let sender = std::thread::Builder::new()
            .name("nova-local-stt".into())
            .spawn(move || transcription_worker(receiver))
            .map(|_| sender)
            .map_err(|error| {
                eprintln!("Could not start NOVA's local speech worker: {error}");
            })
            .ok();
        Self { sender }
    }
}

impl SpeechTranscriber {
    pub fn transcribe(
        &self,
        model_path: PathBuf,
        samples: Vec<f32>,
    ) -> Result<mpsc::Receiver<Result<SpeechTranscription, SpeechError>>, String> {
        let (reply, receiver) = mpsc::channel();
        self.sender
            .as_ref()
            .ok_or_else(|| "The local speech worker could not be started.".to_string())?
            .send(TranscriptionJob {
                model_path,
                samples,
                reply,
            })
            .map_err(|_| "The local speech worker stopped unexpectedly.".to_string())?;
        Ok(receiver)
    }
}

fn transcription_worker(receiver: mpsc::Receiver<TranscriptionJob>) {
    let mut cached: Option<(PathBuf, OfflineRecognizer)> = None;
    while let Ok(job) = receiver.recv() {
        let result = transcribe_with_cache(&mut cached, &job.model_path, &job.samples);
        let _ = job.reply.send(result);
    }
}

fn model_files(model_directory: &Path) -> Option<(PathBuf, PathBuf, PathBuf)> {
    let bundled = (
        model_directory.join(ENCODER_FILE),
        model_directory.join(DECODER_FILE),
        model_directory.join(TOKENS_FILE),
    );
    if bundled.0.is_file() && bundled.1.is_file() && bundled.2.is_file() {
        return Some(bundled);
    }
    let entries = std::fs::read_dir(model_directory).ok()?;
    let mut encoder = None;
    let mut decoder = None;
    let mut tokens = None;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if name.ends_with("encoder.int8.onnx") || name == "encoder.onnx" {
            encoder.get_or_insert(path);
        } else if name.ends_with("decoder.int8.onnx") || name == "decoder.onnx" {
            decoder.get_or_insert(path);
        } else if name.ends_with("tokens.txt") {
            tokens.get_or_insert(path);
        }
    }
    Some((encoder?, decoder?, tokens?))
}

pub fn model_directory_available(model_directory: &Path) -> bool {
    model_files(model_directory).is_some()
}

fn load_recognizer(model_directory: &Path) -> Result<OfflineRecognizer, SpeechError> {
    let (encoder, decoder, tokens) = model_files(model_directory).ok_or_else(|| {
        SpeechError::new(
            SpeechErrorCode::ModelMissing,
            "The configured speech model needs one Whisper encoder, decoder, and tokens file.",
        )
    })?;
    let english_only = model_directory
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().contains(".en"));

    let mut config = OfflineRecognizerConfig::default();
    config.model_config.whisper = OfflineWhisperModelConfig {
        encoder: Some(encoder.to_string_lossy().into_owned()),
        decoder: Some(decoder.to_string_lossy().into_owned()),
        // English-only exports require English. Multilingual Whisper exports
        // use local auto-detection so English, Filipino, and Taglish can coexist.
        language: english_only.then(|| "en".into()),
        task: Some("transcribe".into()),
        tail_paddings: -1,
        enable_token_timestamps: false,
        enable_segment_timestamps: false,
    };
    config.model_config.tokens = Some(tokens.to_string_lossy().into_owned());
    config.model_config.num_threads = std::thread::available_parallelism()
        .map(|count| count.get().clamp(1, 4) as i32)
        .unwrap_or(2);
    config.model_config.provider = Some("cpu".into());
    config.decoding_method = Some("greedy_search".into());

    OfflineRecognizer::create(&config).ok_or_else(|| {
        SpeechError::new(
            SpeechErrorCode::EngineFailure,
            "Could not load the local Whisper model.",
        )
    })
}

fn transcribe_with_cache(
    cached: &mut Option<(PathBuf, OfflineRecognizer)>,
    model_directory: &Path,
    samples: &[f32],
) -> Result<SpeechTranscription, SpeechError> {
    if samples.len() < MIN_TRANSCRIPTION_SAMPLES {
        return Err(SpeechError::new(
            SpeechErrorCode::NoSpeech,
            "No complete spoken command was detected.",
        ));
    }
    if samples.len() > MAX_TRANSCRIPTION_SAMPLES {
        return Err(SpeechError::new(
            SpeechErrorCode::Timeout,
            "The spoken command exceeded the ten-second limit.",
        ));
    }

    let rms =
        (samples.iter().map(|sample| sample.powi(2)).sum::<f32>() / samples.len() as f32).sqrt();
    if !rms.is_finite() || rms < 0.004 {
        return Err(SpeechError::new(
            SpeechErrorCode::NoSpeech,
            "No spoken command could be transcribed.",
        ));
    }

    let reload = cached
        .as_ref()
        .is_none_or(|(cached_path, _)| cached_path != model_directory);
    if reload {
        *cached = Some((
            model_directory.to_path_buf(),
            load_recognizer(model_directory)?,
        ));
    }
    let recognizer = &cached.as_ref().expect("speech recognizer was loaded").1;
    let stream = recognizer.create_stream();
    stream.accept_waveform(16_000, samples);
    recognizer.decode(&stream);
    let result = stream.get_result().ok_or_else(|| {
        SpeechError::new(
            SpeechErrorCode::EngineFailure,
            "The local Whisper engine returned no recognition result.",
        )
    })?;

    validate_transcript(result.text.trim(), &result.tokens, rms)
}

fn validate_transcript(
    text: &str,
    tokens: &[String],
    rms: f32,
) -> Result<SpeechTranscription, SpeechError> {
    let normalized = text
        .trim_matches(|character| character == '[' || character == ']')
        .trim()
        .to_ascii_lowercase();
    if text.is_empty()
        || normalized == "blank_audio"
        || normalized == "silence"
        || normalized == "no speech"
    {
        return Err(SpeechError::new(
            SpeechErrorCode::NoSpeech,
            "No spoken command could be transcribed.",
        ));
    }

    let alphanumeric_count = text
        .chars()
        .filter(|character| character.is_alphanumeric())
        .count();
    let unknown_tokens = tokens
        .iter()
        .filter(|token| token.contains("<unk>"))
        .count();
    if alphanumeric_count < 2 || tokens.is_empty() || unknown_tokens * 2 >= tokens.len() {
        return Err(SpeechError::new(
            SpeechErrorCode::LowConfidence,
            "Speech was detected, but the local transcript was not reliable enough.",
        ));
    }

    // sherpa-onnx's Whisper result does not expose calibrated token probabilities.
    // This bounded quality score reflects signal level and token validity; it is not
    // displayed as model accuracy.
    let signal_quality = (rms / 0.04).clamp(0.0, 1.0);
    let token_quality = 1.0 - unknown_tokens as f32 / tokens.len() as f32;
    Ok(SpeechTranscription {
        text: text.to_string(),
        confidence: 0.3 * signal_quality + 0.7 * token_quality,
    })
}
pub fn effective_model_path(app: &AppHandle) -> Result<PathBuf, String> {
    if let Some(configured) = crate::settings::speech_model_path(app)? {
        return Ok(PathBuf::from(configured));
    }

    let bundled = app
        .path()
        .resource_dir()
        .map_err(|error| format!("Could not locate NOVA resources: {error}"))?
        .join(SPEECH_RESOURCE_DIRECTORY)
        .join(DEFAULT_MODEL_DIRECTORY);
    if model_directory_available(&bundled) {
        return Ok(bundled);
    }

    Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(SPEECH_RESOURCE_DIRECTORY)
        .join(DEFAULT_MODEL_DIRECTORY))
}

pub fn engine_info(app: &AppHandle) -> Result<SpeechEngineInfo, String> {
    let model_path = effective_model_path(app)?;
    Ok(SpeechEngineInfo {
        engine: SPEECH_ENGINE_NAME,
        model: DEFAULT_MODEL_NAME,
        model_available: model_directory_available(&model_path),
        model_path: model_path.to_string_lossy().into_owned(),
        language: if model_path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.to_ascii_lowercase().contains(".en"))
        {
            "English"
        } else {
            "Auto (multilingual model)"
        },
        sample_rate: 16_000,
    })
}

#[tauri::command]
pub fn get_speech_engine_info(app: AppHandle) -> Result<SpeechEngineInfo, String> {
    engine_info(&app)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_returns_a_typed_error() {
        let mut cache = None;
        let error = transcribe_with_cache(
            &mut cache,
            Path::new(r"Z:\definitely-missing\model.bin"),
            &vec![0.1; MIN_TRANSCRIPTION_SAMPLES],
        )
        .unwrap_err();
        assert_eq!(error.code, SpeechErrorCode::ModelMissing);
    }

    #[test]
    fn empty_audio_is_rejected_before_model_loading() {
        let mut cache = None;
        let error = transcribe_with_cache(&mut cache, Path::new("missing.bin"), &[]).unwrap_err();
        assert_eq!(error.code, SpeechErrorCode::NoSpeech);
    }

    #[test]
    fn overlong_audio_is_rejected_before_model_loading() {
        let mut cache = None;
        let error = transcribe_with_cache(
            &mut cache,
            Path::new("missing.bin"),
            &vec![0.0; MAX_TRANSCRIPTION_SAMPLES + 1],
        )
        .unwrap_err();
        assert_eq!(error.code, SpeechErrorCode::Timeout);
    }

    #[test]
    fn voice_visualization_tracks_speech_and_settles_in_silence() {
        let mut capture = CommandCapture::default();
        feed(&mut capture, 0.002, 4);
        assert_eq!(capture.voice_level(), 0.0);
        feed(&mut capture, 0.06, 4);
        assert!(capture.voice_level() > 0.0 && capture.voice_level() <= 1.0);
        feed(&mut capture, 0.0, 4);
        assert_eq!(capture.voice_level(), 0.0);
        capture.process(&[]);
        assert_eq!(capture.voice_level(), 0.0);
    }
    fn feed(capture: &mut CommandCapture, amplitude: f32, frames: usize) -> CaptureOutcome {
        let mut outcome = CaptureOutcome::Continue;
        let waveform = (0..CAPTURE_FRAME_SAMPLES)
            .map(|index| {
                if index % 2 == 0 {
                    amplitude
                } else {
                    -amplitude
                }
            })
            .collect::<Vec<_>>();
        for _ in 0..frames {
            outcome = capture.process(&waveform);
            if !matches!(outcome, CaptureOutcome::Continue) {
                break;
            }
        }
        outcome
    }

    #[test]
    fn silence_keeps_listening_with_bounded_buffers() {
        let mut capture = CommandCapture::default();
        assert!(matches!(feed(&mut capture, 0.0, 4000), CaptureOutcome::Continue));
        assert!(!capture.speech_started);
        assert!(capture.pre_roll.len() <= PRE_ROLL_SAMPLES);
        assert!(capture.captured.is_empty());
    }

    #[test]
    fn neural_vad_rejects_noise_and_tones() {
        let model = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/vad/silero_vad.onnx");
        for kind in 0..4 {
            let mut capture = CommandCapture::with_vad(&model).expect("bundled VAD loads");
            let mut random = 123456789u32;
            for frame in 0..200 {
                let samples = (0..480).map(|i| {
                    let t = (frame * 480 + i) as f32 / 16000.0;
                    random = random.wrapping_mul(1664525).wrapping_add(1013904223);
                    match kind {
                        0 => 0.0,
                        1 => (random as f32 / u32::MAX as f32 - 0.5) * 0.12,
                        2 => (t * 120.0 * std::f32::consts::TAU).sin() * 0.08,
                        _ => if frame % 50 == 0 && i < 10 { 0.8 } else { 0.0 },
                    }
                }).collect::<Vec<_>>();
                assert!(matches!(capture.process(&samples), CaptureOutcome::Continue));
            }
            assert!(!capture.speech_started, "noise case {kind} activated capture");
        }
    }

    #[test]
    #[ignore = "requires NOVA_VAD_TEST_WAV containing 16 kHz speech"]
    fn neural_vad_captures_speech_after_noise_and_waits_for_silence() {
        let model = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/vad/silero_vad.onnx");
        let mut capture = CommandCapture::with_vad(&model).expect("bundled VAD loads");
        let wave = sherpa_onnx::Wave::read(&std::env::var("NOVA_VAD_TEST_WAV").unwrap()).unwrap();
        assert_eq!(wave.sample_rate(), 16000);
        let mut detected = false;
        let mut complete = false;
        for samples in wave.samples().chunks(480).chain(std::iter::repeat(&[0.0f32; 480][..]).take(40)) {
            match capture.process(samples) {
                CaptureOutcome::Complete(audio) => {
                    assert!(detected);
                    assert!(audio.len() >= MIN_TRANSCRIPTION_SAMPLES);
                    complete = true;
                    break;
                }
                CaptureOutcome::Error(error) => panic!("speech capture failed: {:?}", error),
                CaptureOutcome::Continue => { detected |= capture.voice_level() > 0.0; }
            }
        }
        assert!(detected, "speech did not activate VAD");
        assert!(complete, "speech did not finish after silence");
    }

    #[test]
    fn missing_vad_fails_closed() {
        assert!(CommandCapture::with_vad(Path::new("missing-vad-model.onnx")).is_err());
    }

    #[test]
    fn steady_background_noise_does_not_start_capture() {
        let mut capture = CommandCapture::default();
        let outcome = feed(&mut capture, MIN_SPEECH_RMS_THRESHOLD / 2.0, 40);
        assert!(matches!(outcome, CaptureOutcome::Continue));
        assert!(!capture.speech_started);
    }

    #[test]
    fn dc_offset_does_not_start_capture() {
        let mut capture = CommandCapture::default();
        let mut outcome = CaptureOutcome::Continue;
        for _ in 0..40 {
            outcome = capture.process(&vec![0.03; CAPTURE_FRAME_SAMPLES]);
        }
        assert!(matches!(outcome, CaptureOutcome::Continue));
        assert!(!capture.speech_started);
    }

    #[test]
    fn a_natural_pause_does_not_end_a_compound_command() {
        let mut capture = CommandCapture::default();
        assert!(matches!(
            feed(&mut capture, 0.06, 12),
            CaptureOutcome::Continue
        ));
        assert!(matches!(
            feed(&mut capture, 0.0, 20),
            CaptureOutcome::Continue
        ));
        assert!(matches!(
            feed(&mut capture, 0.06, 12),
            CaptureOutcome::Continue
        ));
        assert!(capture.speech_started);
    }

    #[test]
    fn trailing_silence_completes_a_spoken_command() {
        let mut capture = CommandCapture::default();
        assert!(matches!(
            feed(&mut capture, 0.08, 14),
            CaptureOutcome::Continue
        ));
        let frames = END_SILENCE_SAMPLES / CAPTURE_FRAME_SAMPLES + 1;
        let CaptureOutcome::Complete(samples) = feed(&mut capture, 0.0, frames) else {
            panic!("trailing silence should complete the command");
        };
        assert!(samples.len() >= MIN_VOICED_SAMPLES);
        assert!(samples.len() <= MAX_COMMAND_SAMPLES);
    }

    #[test]
    fn long_speech_waits_for_silence_before_reporting_limit() {
        let mut capture = CommandCapture::default();
        assert!(matches!(feed(&mut capture, 0.06, 500), CaptureOutcome::Continue));
        assert!(capture.captured.len() <= MAX_COMMAND_SAMPLES);
        let CaptureOutcome::Error(error) = feed(&mut capture, 0.0, 40) else {
            panic!("long speech should report the size limit after speech stops");
        };
        assert_eq!(error.code, SpeechErrorCode::Timeout);
    }

    #[test]
    fn consecutive_command_captures_have_independent_state() {
        for _ in 0..2 {
            let mut capture = CommandCapture::default();
            assert!(matches!(
                feed(&mut capture, 0.08, 14),
                CaptureOutcome::Continue
            ));
            let frames = END_SILENCE_SAMPLES / CAPTURE_FRAME_SAMPLES + 1;
            assert!(matches!(
                feed(&mut capture, 0.0, frames),
                CaptureOutcome::Complete(_)
            ));
        }
    }
    #[test]
    fn invalid_transcript_is_reported_as_low_confidence() {
        let error = validate_transcript("?", &["<unk>".into()], 0.02).unwrap_err();
        assert_eq!(error.code, SpeechErrorCode::LowConfidence);
    }

    #[test]
    fn silence_marker_is_reported_as_no_speech() {
        let error = validate_transcript("[SILENCE]", &["silence".into()], 0.02).unwrap_err();
        assert_eq!(error.code, SpeechErrorCode::NoSpeech);
    }

    #[test]
    #[ignore = "requires NOVA_SPEECH_TEST_WAV and performs real local model inference"]
    fn local_whisper_transcribes_known_english_audio() {
        let wave_path = std::env::var("NOVA_SPEECH_TEST_WAV")
            .expect("set NOVA_SPEECH_TEST_WAV to a local 16 kHz WAV file");
        let wave = sherpa_onnx::Wave::read(&wave_path).expect("read validation WAV");
        assert_eq!(wave.sample_rate(), 16_000);
        let model_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(SPEECH_RESOURCE_DIRECTORY)
            .join(DEFAULT_MODEL_DIRECTORY);
        let mut cache = None;
        let result = transcribe_with_cache(&mut cache, &model_directory, wave.samples())
            .expect("transcribe known English audio");
        assert!(
            result.text.to_ascii_lowercase().contains("early nightfall"),
            "unexpected transcript: {}",
            result.text
        );
    }
}
