export interface NovaStatus {
  /** Persisted application setting enforced by native assistant entry points. */
  enabled: boolean;
  wakePhrase: string;
  localMode: boolean;
  version: string;
  stage: "Development release";
}
