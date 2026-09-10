# Whisper Tiny English INT8 model provenance

Source archive:
https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-whisper-tiny.en.tar.bz2

Model documentation:
https://k2-fsa.github.io/sherpa/onnx/pretrained_models/whisper/tiny.en.html

Only the quantized encoder, quantized decoder, and tokens file are bundled. The full-precision duplicate models and sample audio are excluded.

SHA-256:

- `tiny.en-encoder.int8.onnx`: `0CE578B827C94A961AACB8FA14B02F096504B337E5C94BE37C36238CBE3E8BC6`
- `tiny.en-decoder.int8.onnx`: `06C0E6FF6348D427E51839219D1C886C18CFDF411E629E33F5E1679BFF9C1527`
- `tiny.en-tokens.txt`: `306CD27F03C1A714ECA7108E03D66B7DC042ABE8C258B44C199A7ED9838DD930`

Whisper code and model weights are released under the MIT License. sherpa-onnx is Apache-2.0 licensed. Review the upstream repositories for complete license notices.