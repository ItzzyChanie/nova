import type { NovaPage } from "../../types/navigation";

const paths = {
  overview: "M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z",
  history: "M3 11a9 9 0 1 1 2.5 7 M3 4v7h7 M12 7v5l3 2",
  skills: "m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5Z",
  voice:
    "M9 5a3 3 0 0 1 6 0v7a3 3 0 0 1-6 0Z M5 10v2a7 7 0 0 0 14 0v-2 M12 19v3 M9 22h6",
  privacy: "m12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6Z m-4 9 3 3 5-6",
  developer: "M3 4h18v16H3Z M3 8h18 m-14 4 3 2-3 2 m6 0h4",
  files: "M3 7V4h6l3 3h9v13H3Z",
  system: "M3 4h18v13H3Z M12 17v4 M8 21h8",
  about: "M12 8h.01 M12 11v6 M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0",
  arrow: "M5 12h14 m-5-5 5 5-5 5",
  power: "M12 3v9 M7 5a9 9 0 1 0 10 0",
} satisfies Record<NovaPage | "arrow" | "power", string>;

export function Icon({
  name,
  className = "",
}: {
  name: keyof typeof paths;
  className?: string;
}) {
  return (
    <svg
      className={`icon ${className}`}
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name]} />
    </svg>
  );
}
