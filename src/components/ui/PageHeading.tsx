interface PageHeadingProps {
  title: string;
  description: string;
  preview?: string;
}

export function PageHeading({ title, description, preview }: PageHeadingProps) {
  return (
    <header className="page-heading">
      <h1 id="page-title" tabIndex={-1}>{title}</h1>
      <p>{description}</p>
      {preview && <p id="preview-note" className="preview-note"><span>UI preview</span> {preview}</p>}
    </header>
  );
}
