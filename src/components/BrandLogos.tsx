import React from 'react';
import type { AgentKind } from '../lib/ipc';

interface LogoProps extends React.SVGProps<SVGSVGElement> {
  size?: number;
}

// Стилизованные знаки нейронок: все рисуются currentColor,
// чтобы наследовать цвет исполнителя (--acc / --codex / --agy).

/** Claude — лучистая «звезда» Anthropic. */
export const ClaudeLogo: React.FC<LogoProps> = ({ size = 16, ...props }) => {
  const rays = Array.from({ length: 12 }, (_, i) => {
    const a = (i * 30 * Math.PI) / 180;
    const len = i % 3 === 0 ? 9.5 : i % 3 === 1 ? 7.6 : 8.6;
    const x2 = 12 + Math.cos(a) * len;
    const y2 = 12 + Math.sin(a) * len;
    const x1 = 12 + Math.cos(a) * 2.2;
    const y1 = 12 + Math.sin(a) * 2.2;
    return <line key={i} x1={x1} y1={y1} x2={x2} y2={y2} />;
  });
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.6"
      strokeLinecap="round"
      aria-hidden="true"
      {...props}
    >
      {rays}
    </svg>
  );
};

/** Codex — шестилепестковый узел OpenAI. */
export const CodexLogo: React.FC<LogoProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
    {...props}
  >
    {Array.from({ length: 6 }, (_, i) => (
      <path
        key={i}
        d="M18.9 6.6 L18.9 14.4 L14.6 12.9"
        transform={`rotate(${i * 60} 12 12)`}
      />
    ))}
  </svg>
);

/** Antigravity — четырёхлучевая искра Gemini. */
export const AgyLogo: React.FC<LogoProps> = ({ size = 16, ...props }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true" {...props}>
    <path d="M12 1.5C12 7.6 7.6 12 1.5 12 7.6 12 12 16.4 12 22.5 12 16.4 16.4 12 22.5 12 16.4 12 12 7.6 12 1.5Z" />
  </svg>
);

export type LogoKind = AgentKind | 'antigravity' | 'agy';

/** Логотип по виду агента: 'codex', 'gemini' | 'agy' | 'antigravity', иначе Claude. */
export const AgentLogo: React.FC<LogoProps & { agent: LogoKind }> = ({ agent, ...props }) => {
  if (agent === 'codex') return <CodexLogo {...props} />;
  if (agent === 'gemini' || agent === 'agy' || agent === 'antigravity') return <AgyLogo {...props} />;
  return <ClaudeLogo {...props} />;
};
