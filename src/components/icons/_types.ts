import type { LucideProps } from "lucide-react";
import type { RefAttributes } from "react";

export type IconProps = Omit<LucideProps, "ref"> & RefAttributes<SVGSVGElement>;
