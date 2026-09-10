# Silero voice activity detector

Source: https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx
Documentation: https://k2-fsa.github.io/sherpa/onnx/vad/silero-vad.html
Upstream: https://github.com/snakers4/silero-vad (MIT license)
SHA-256: 9e2449e1087496d8d4caba907f23e0bd3f78d91fa552479bb9c23ac09cbb1fd6

Bundled for offline detection at 16 kHz. No microphone audio is sent to a service.
The 0.65 speech threshold is intentionally conservative. VAD distinguishes
speech-like audio from noise; it cannot verify a live person or exclude voices
from a television. A missing model fails closed instead of falling back to volume.
