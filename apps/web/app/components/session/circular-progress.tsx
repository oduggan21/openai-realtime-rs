import { cn } from "@revlentless/ui/lib/utils";

export type CircularProgressProps = {
  value?: number;
  size?: number;
  strokeWidth?: number;
  className?: string;
};

export default function CircularProgress({
  value = 0,
  size = 64,
  strokeWidth = 8,
  className,
}: CircularProgressProps) {
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const clamped = Math.max(0, Math.min(100, value));
  const dash = (clamped / 100) * circumference;

  return (
    <svg
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      className={cn("block", className)}
      role="img"
      aria-label={`Progress ${clamped}%`}
    >
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        strokeWidth={strokeWidth}
        stroke="hsl(var(--muted-foreground) / 0.2)"
        fill="none"
      />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        stroke="hsl(142 76% 36%)"
        fill="none"
        strokeDasharray={`${dash} ${circumference - dash}`}
        transform={`rotate(-90 ${size / 2} ${size / 2})`}
      />
      <text
        x="50%"
        y="50%"
        dominantBaseline="central"
        textAnchor="middle"
        fontSize={Math.max(10, size * 0.28)}
        fill="currentColor"
        className="font-semibold"
      >
        {clamped}%
      </text>
    </svg>
  );
}
