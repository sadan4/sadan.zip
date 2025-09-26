import { Text, type TextProps, type TextTags } from "@/components/Text";

import type { TypewriterFrame, TypewriterSource } from ".";

import type { ReactNode } from "react";

export function *defaultEraser(prev: ReactNode, delay = 50): Generator<TypewriterFrame, void, ReactNode> {
    if (typeof prev !== "string") {
        yield {
            component: prev,
            nextDelay: 0,
        };
        return;
    }

    let cur = prev;

    while (cur.length > 0) {
        yield {
            component: `${cur = cur.substring(0, cur.length - 1)}|`,
            nextDelay: delay,
        };
    }
    yield {
        component: "",
        nextDelay: delay * 4,
    };
    return;
}

export function stringTypewriter(nextDelay: number, string: string): TypewriterSource {
    return {
        *type() {
            for (let i = 1; i <= string.length; i++) {
                yield {
                    component: `${string.substring(0, i)}|`,
                    nextDelay,
                };
            }
            yield {
                component: string,
                nextDelay,
            };
        },
        erase: defaultEraser,
    };
}

export function textComponentTypewriter(
    nextDelay: number,
    str: string,
    extraProps: Omit<TextProps<"span">, "children"> = {},
): TypewriterSource {
    return {
        *type() {
            for (let i = 1; i <= str.length; i++) {
                yield {
                    component: <Text {...extraProps}>{str.substring(0, i)}|</Text>,
                    nextDelay,
                };
            }
            yield {
                component: <Text {...extraProps}>{str}</Text>,
                nextDelay,
            };
        },
        erase: makeTextComponentEraser(str, nextDelay, extraProps),
    };
}

export function makeTextComponentEraser(
    prevStr: string,
    nextDelay: number,
    extraProps: TextProps<TextTags>,
) {
    return function *(): Generator<TypewriterFrame, void, ReactNode> {
        let cur = prevStr;

        while (cur.length > 0) {
            yield {
                component: <Text {...extraProps}>{cur = cur.substring(0, cur.length - 1)}|</Text>,
                nextDelay,
            };
        }
        yield {
            component: (
                <Text
                    {...extraProps}
                    children=""
                />
            ),
            nextDelay,
        };
    };
}

export interface TypewriterImage {
    htmlTag: string;
    alt: string;
    href: string;
}
export function imageTypewriter(image: TypewriterImage, extraProps: TextProps<TextTags>): TypewriterSource {
    return {
        *type() {
            for (let i = 1; i <= image.htmlTag.length; i++) {
                yield {
                    component: <Text {...extraProps}>{image.htmlTag.substring(0, i)}|</Text>,
                    nextDelay: 35,
                };
            }
            yield {
                component: (
                    <img
                        className="h-32 w-32"
                        src={image.href}
                        alt={image.alt}
                    />
                ),
                nextDelay: 0,
            };
        },
        erase: makeTextComponentEraser(image.htmlTag, 35, extraProps),
    };
}
