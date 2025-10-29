// This file has been automatically migrated to valid ESM format by Storybook.
import { fileURLToPath } from "node:url";
import type { StorybookConfig } from "@storybook/react-vite";

import { join, dirname } from "node:path";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const config: StorybookConfig = {
    stories: [
        "../src/**/*.mdx",
        "../src/**/*.stories.@(js|jsx|mjs|ts|tsx)",
    ],
    addons: [
        "@storybook/addon-docs",
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
