import type { PropsWithChildren } from "react";

export interface LinkProps extends PropsWithChildren {
  href: HTMLAnchorElement["href"];
  target?:
    | `_${"blank" | "self" | "parent" | "top"}`
    | (HTMLAnchorElement["target"] & Record<never, never>);
}
export default function Link({ target = "_blank", href, children }: LinkProps) {
  return (
    <a href={href} target={target}>
      {children}
    </a>
  );
}

export function ThemeLink() {
  return (
    <Link href="https://github.com/enkia/tokyo-night-vscode-theme/tree/master">
      Color Scheme
    </Link>
  );
}

export function SourceLink() {
  return <Link href="https://github.com/sadan4/sadan.zip">Source Code</Link>;
}
