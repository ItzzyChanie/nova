export interface NovaStatus {
  /** Frontend preview preference, not the state of a running assistant engine. */
  enabled: boolean;
  wakePhrase: string;
  localMode: boolean;
  version: string;
  stage: "Foundation";
}
