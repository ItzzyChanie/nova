export interface SpeechEngineInfo {
  engine: string;
  model: string;
  modelPath: string;
  modelAvailable: boolean;
  language: string;
  sampleRate: number;
}