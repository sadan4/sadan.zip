import type { StorybookConfig } from "@storybook/react-vite";

import { join } from "node:path";

const config: StorybookConfig = {
    stories: [
        "../src/**/*.mdx",
        "../src/**/*.stories.@(js|jsx|mjs|ts|tsx)",
    ],
    addons: [
        "@storybook/addon-docs",
        "@storybook/addon-onboarding",
        "@storybook/addon-a11y",
        "@storybook/addon-vitest",
        "@storybook/addon-themes",
    ],
    framework: {
        name: "@storybook/react-vite",
        options: {
            builder: {
                viteConfigPath: join(__dirname, "..", "vite.config.ts"),
            },
        },
    },
    typescript: {
        reactDocgen: "react-docgen-typescript",
        reactDocgenTypescriptOptions: {
            tsconfigPath: join(__dirname, "..", "tsconfig.app.json"),
            shouldExtractValuesFromUnion: true,
        },

    },
};

export default config;
