import { Boilerplate } from "@/components/Boilerplate";
import { CustomCursorContext } from "@/components/Cursor/context";
import type { Preview } from "@storybook/react-vite";

import cssUrl from "../src/index.css?url";

import { themes } from "storybook/theming";

// INSANITY https://github.com/storybookjs/storybook/issues/23990
const styleEl = function (): HTMLLinkElement {
    const ID = "index-style-storybook-workaround";
    let cur = document.getElementById(ID) as HTMLLinkElement | null;

    if (!cur) {
        cur = document.createElement("link");
        cur.id = ID;
        document.head.prepend(cur);
    }
    cur.rel = "stylesheet";
    return cur;
}();

styleEl.href = cssUrl;

import { createPortal } from "react-dom";

const preview: Preview = {
    parameters: {
        controls: {
        },
        a11y: {
            // 'todo' - show a11y violations in the test UI only
            // 'error' - fail CI on a11y violations
            // 'off' - skip a11y checks entirely
            test: "todo",
        },
        docs: {
            theme: themes.dark,
        },
    },
    tags: ["autodocs"],
    globalTypes: {
        customCursor: {
            description: "Enable the custom cursor",
            toolbar: {
                icon: "pointerdefault",
                title: "Custom Cursor",
                items: [
                    {
                        value: false,
                        right: "❌",
                        title: "Disabled",
                    },
                    {
                        value: true,
                        right: "✔️",
                        title: "Enabled",
                    },
                ],
            },
        },
    },
    initialGlobals: {
        customCursor: false,
    },
    decorators: [
        (storyFn, context) => {
            return (
                <CustomCursorContext value={!context.globals.customCursor}>
                    {createPortal(<Boilerplate />, document.body)}
                    {storyFn()}
                </CustomCursorContext>
            );
        },
    ],
};

export default preview;
