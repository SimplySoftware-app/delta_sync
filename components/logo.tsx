export function Logo({ className }: { className?: string }) {
  return (
    <svg
      width="28"
      height="28"
      viewBox="0 0 32 32"
      fill="none"
      aria-hidden="true"
      className={className}
    >
      <path
        fill="currentColor"
        d="M16 3.5 29 27H3L16 3.5Zm0 6.2L8.2 24.5h15.6L16 9.7Z"
      />
    </svg>
  )
}
