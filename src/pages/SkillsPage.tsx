import { Card } from "../components/ui/Card";
import { PageHeading } from "../components/ui/PageHeading";
import { Toggle } from "../components/ui/Toggle";
import { skillDefinitions, type SettingsPageProps } from "../types/settings";

export function SkillsPage({ settings, onChange }: SettingsPageProps) {
  return (
    <>
      <PageHeading
        title="Skills"
        description="What NOVA is allowed to do. Turn off anything you don't want it touching."
        preview="Configuration only. No permissions are granted and no actions can run. Preferences reset on reload."
      />
      <div className="card-grid">
        {skillDefinitions.map(({ id, title, description }) => (
          <Card
            key={id}
            title={title}
            description={description}
            control={<Toggle label={title} checked={settings.skills[id]} onChange={(checked) => onChange({ skills: { ...settings.skills, [id]: checked } })} />}
          />
        ))}
      </div>
    </>
  );
}
