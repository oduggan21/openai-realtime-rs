import { Check, Circle, HelpCircle } from "lucide-react";
import { cn } from "@revlentless/ui/lib/utils";

export type IconBadgeProps = {
  label?: string;
  state?: "pending" | "covered" | "questioned";
};

export default function IconBadge({
  label = "Label",
  state = "pending",
}: IconBadgeProps) {
  const { Icon, styles } = getVisual(state);
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1 rounded-full border px-2 py-1 text-xs",
        styles.container
      )}
      aria-label={`${label}: ${state}`}
      title={`${label}: ${capitalize(state)}`}
    >
      <Icon className={cn("h-3.5 w-3.5", styles.icon)} aria-hidden="true" />
      <span className={cn(styles.text)}>{label}</span>
    </span>
  );
}

function getVisual(state: "pending" | "covered" | "questioned") {
  switch (state) {
    case "covered":
      return {
        Icon: Check,
        styles: {
          container: "border-emerald-200 bg-emerald-50",
          icon: "text-emerald-600",
          text: "text-emerald-700",
        },
      };
    case "questioned":
      return {
        Icon: HelpCircle,
        styles: {
          container: "border-amber-200 bg-amber-50",
          icon: "text-amber-600",
          text: "text-amber-700",
        },
      };
    default:
      return {
        Icon: Circle,
        styles: {
          container: "border-muted bg-background",
          icon: "text-muted-foreground",
          text: "text-muted-foreground",
        },
      };
  }
}

function capitalize(s: string) {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
