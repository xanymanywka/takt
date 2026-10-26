import React from 'react';

interface IconProps extends React.SVGProps<SVGSVGElement> {
  size?: number;
}

export const SunIcon: React.FC<IconProps> = ({ size = 16, className, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    {...props}
  >
    <circle cx="8" cy="8" r="3.2" />
    <path d="M8 1.5v1.6M8 12.9v1.6M1.5 8h1.6M12.9 8h1.6M3.4 3.4l1.1 1.1M11.5 11.5l1.1 1.1M3.4 12.6l1.1-1.1M11.5 4.5l1.1-1.1" />
  </svg>
);

export const MoonIcon: React.FC<IconProps> = ({ size = 16, className, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    className={className}
    {...props}
  >
    <path d="M13.5 9.8A6 6 0 0 1 6.2 2.5a6 6 0 1 0 7.3 7.3Z" />
  </svg>
);

export const PlusIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    {...props}
  >
    <path d="M8 3v10M3 8h10" />
  </svg>
);

export const SendIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.8"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M8 13V3M3.5 7.5 8 3l4.5 4.5" />
  </svg>
);

export const CheckIcon: React.FC<IconProps> = ({ size = 12, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 12 12"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.6"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M2 6.5 4.6 9 10 3.5" />
  </svg>
);

export const DotIcon: React.FC<IconProps> = ({ size = 12, ...props }) => (
  <svg width={size} height={size} viewBox="0 0 12 12" fill="currentColor" {...props}>
    <circle cx="6" cy="6" r="3" />
  </svg>
);

export const CircleTodoIcon: React.FC<IconProps> = ({ size = 12, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 12 12"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.4"
    {...props}
  >
    <circle cx="6" cy="6" r="3.3" />
  </svg>
);

export const ServerHostIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    {...props}
  >
    <rect x="2" y="3" width="12" height="10" rx="2" />
    <path d="m5 7 2 1.5L5 10M8.5 10H11" />
  </svg>
);

export const GhIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    {...props}
  >
    <path d="M8 2.5c-3 0-5.5 2.3-5.5 5.3 0 2.3 1.5 4.3 3.6 5v-1.9c-1.3.3-1.7-.6-1.7-.6-.3-.6-.6-.8-.6-.8-.4-.3.1-.3.1-.3.5 0 .8.6.8.6.4.8 1.2.6 1.5.4 0-.3.2-.6.3-.7-1.4-.2-2.6-.7-2.6-2.8 0-.6.2-1.1.6-1.5 0-.2-.3-.8 0-1.6 0 0 .5-.2 1.6.6a5 5 0 0 1 2.8 0c1.1-.8 1.6-.6 1.6-.6.3.8 0 1.4 0 1.6.4.4.6.9.6 1.5 0 2.1-1.2 2.6-2.6 2.8.2.2.4.6.4 1.2v2.1c2.1-.7 3.6-2.7 3.6-5 0-3-2.5-5.3-5.5-5.3Z" />
  </svg>
);

export const CodeRabbitIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    {...props}
  >
    <path d="M3 4h10M3 8h10M3 12h6" />
  </svg>
);

export const WranglerIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    {...props}
  >
    <path d="M2 12 8 3l6 9H2Z" />
  </svg>
);

export const ArrowRightIcon: React.FC<IconProps> = ({ size = 14, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 14 14"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.8"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M3 7h8M7.5 3.5 11 7l-3.5 3.5" />
  </svg>
);

export const TerminalIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <rect x="2" y="3" width="12" height="10" rx="2" />
    <path d="m5 6 2 2-2 2M9 10h2" />
  </svg>
);

export const GearIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <circle cx="8" cy="8" r="2.3" />
    <path d="M8 1.8v1.7M8 12.5v1.7M1.8 8h1.7M12.5 8h1.7M3.6 3.6l1.2 1.2M11.2 11.2l1.2 1.2M3.6 12.4l1.2-1.2M11.2 4.8l1.2-1.2" />
  </svg>
);

export const ArrowLeftIcon: React.FC<IconProps> = ({ size = 14, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 14 14"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.8"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M11 7H3M6.5 3.5 3 7l3.5 3.5" />
  </svg>
);

export const PanelIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <rect x="2" y="3" width="12" height="10" rx="2" />
    <path d="M9.5 3v10" />
  </svg>
);

export const ExternalIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M9 2.5h4.5V7M13.5 2.5 7.5 8.5M12 9.5v3a1 1 0 0 1-1 1H3.5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h3" />
  </svg>
);

export const ReloadIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M13 8a5 5 0 1 1-1.5-3.6M13 2.5v3h-3" />
  </svg>
);

export const ChevronRightIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="m6 3.5 4.5 4.5L6 12.5" />
  </svg>
);

export const GlobeIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <circle cx="8" cy="8" r="5.8" />
    <path d="M2.2 8h11.6M8 2.2c1.7 1.6 2.5 3.5 2.5 5.8S9.7 12.2 8 13.8C6.3 12.2 5.5 10.3 5.5 8S6.3 3.8 8 2.2Z" />
  </svg>
);

export const ImageIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <rect x="2" y="3" width="12" height="10" rx="2" />
    <circle cx="6" cy="6.5" r="1.2" />
    <path d="m2.5 12 3.5-3.5 2.5 2.5 2-2 3 3" />
  </svg>
);

export const DocIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M4 1.8h5l3 3v9.4H4z" />
    <path d="M9 1.8v3h3M6 8.5h4M6 11h4" />
  </svg>
);

export const DiffIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M5 2.5v6M2 5.5h6M8 12h6M11 10v4" />
  </svg>
);

export const PhoneIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <rect x="4.5" y="1.8" width="7" height="12.4" rx="1.6" />
    <path d="M7.2 12h1.6" />
  </svg>
);

export const TabletIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <rect x="2.8" y="1.8" width="10.4" height="12.4" rx="1.6" />
    <path d="M7.2 12h1.6" />
  </svg>
);

export const DesktopIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <rect x="1.8" y="2.5" width="12.4" height="8.5" rx="1.4" />
    <path d="M5.5 14h5M8 11v3" />
  </svg>
);

export const MinusIcon: React.FC<IconProps> = ({ size = 16, ...props }) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    strokeWidth="1.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    {...props}
  >
    <path d="M3.5 8h9" />
  </svg>
);
