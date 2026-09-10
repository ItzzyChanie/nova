import { Card } from "../components/ui/Card";
import { PageHeading } from "../components/ui/PageHeading";
import { Toggle } from "../components/ui/Toggle";
import {
  skillDefinitions,
  type NovaSettings,
  type SkillId,
} from "../types/settings";

interface SkillsPageProps {
  settings: NovaSettings;
  settingsReady: boolean;
  savingSkill: SkillId | null;
  error: string | null;
  onPreferenceChange: (skill: SkillId, enabled: boolean) => void;
}

export function SkillsPage({
  settings,
  settingsReady,
  savingSkill,
  error,
  onPreferenceChange,
}: SkillsPageProps) {
  return (
    <>
      <PageHeading
        title="Skills"
        description="Choose what NOVA can help with."
        preview="A disabled preference denies its mapped tools. Enabling one never grants implementation or bypasses each tool's confirm/deny policy."
      />
      {error && (
        <p className="assistant-launch-error" role="alert">
          {error}
        </p>
      )}
      <div className="card-grid">
        {skillDefinitions.map((definition) => {
          const enabled = settings.skills[definition.id];
          const categories =
            definition.toolCategories.length > 0
              ? definition.toolCategories.join(", ")
              : "None";

          return (
            <Card
              key={definition.id}
              title={definition.title}
              description={definition.description}
              control={
                <Toggle
                  label={`${definition.title} permission preference`}
                  checked={enabled}
                  disabled={!settingsReady || savingSkill !== null}
                  onChange={(checked) =>
                    onPreferenceChange(definition.id, checked)
                  }
                />
              }
            >
              <p className="technical value">
                Preference: {enabled ? "Enabled" : "Disabled"}
                {" · "}
                Native permission:{" "}
                {enabled ? definition.enabledPolicy : "Denied by preference"}
              </p>
              <p className="technical tool-mapping">
                Tool categories: {categories}
                {" · "}
                Capability:{" "}
                {definition.implemented ? "Available" : "Not available yet"}
              </p>
            </Card>
          );
        })}
      </div>
    </>
  );
}
