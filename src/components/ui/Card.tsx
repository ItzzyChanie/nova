import { useId, type ReactNode } from "react";

interface CardProps {
  title: string;
  description?: string;
  control?: ReactNode;
  children?: ReactNode;
}

export function Card({ title, description, control, children }: CardProps) {
  const id = useId();

  return (
    <section className="panel feature-card" aria-labelledby={id}>
      <div className="card-heading">
        <h2 id={id}>{title}</h2>
        {control}
      </div>
      {description && <p>{description}</p>}
      {children}
    </section>
  );
}
