import {
  type ComponentProps,
  type PropsWithChildren,
  type ReactNode,
} from "react";
import { joinWith } from "../utils/array";
import { SourceLink, ThemeLink } from "./Links";
import _ from "lodash";
import invariant from "invariant";

export interface FooterProps extends PropsWithChildren {
  className?: string;
}

function FooterSeperator() {
  return " | ";
}

export default function Footer({
  className,
  children: _children,
}: FooterProps) {
  const children: ReactNode[] = Array.isArray(_children)
    ? [..._children]
    : [_children];
  return (
    <div className={className}>{joinWith(children, <FooterSeperator />)}</div>
  );
}

export function DefaultFooter() {
  return (
    <Footer>
      <ThemeLink />
      <SourceLink />
    </Footer>
  );
}

interface FooterContainerProps extends ComponentProps<"div"> {
  footer: ReactNode;
}

export function FooterContainer({
  children,
  footer,
  ...props
}: FooterContainerProps) {
  return (
    <div className="grid grid-rows-[1fr_min-content] w-full h-full">
      <div {...props}>{children}</div>
      <div className="flex justify-center items-center content-center">
        {footer}
      </div>
    </div>
  );
}
