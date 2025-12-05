interface LogoProps {
  className?: string;
  size?: number;
}

export default function Logo({ className = '', size = 32 }: LogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 100 100"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      {/* Art Nouveau inspired flowing curves around the triangle */}
      <path
        d="M50 10 L85 75 L15 75 Z"
        fill="#2c3b4d"
        stroke="#a35139"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      />

      {/* MiddleInner triangle */}
      <path
        d="M50 30 L68.301 66.961 L31.699 66.961 Z"
        fill="none"
        stroke="#a35139"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
        opacity="0.6"
      />
    </svg>
  );
}

