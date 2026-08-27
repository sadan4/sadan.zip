import type { StorybookConfig } from "@storybook/react-vite";
import tailwindcss from "@tailwindcss/vite";
import viteReact, { reactCompilerPreset } from "@vitejs/plugin-react";

import { omt } from "../scripts/vite-plugin-omt.ts";

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { mergeConfig } from "vite";

const __dirname = dirname(fileURLToPath(import.meta.url));
const srcDir = join(__dirname, "..", "src");

const config: StorybookConfig = {
    stories: ["../src/**/*.stories.@(ts|tsx)"],
    framework: {
        name: "@storybook/react-vite",
        options: {
            builder: {
                viteConfigPath: ".storybook/vite.config.ts",
            },
        },
    },
    viteFinal(config) {
        return mergeConfig(config, {
            plugins: [
                tailwindcss(),
                viteReact({
                    babel: {
                        presets: [reactCompilerPreset()],
                    },
                }),
                omt(),
            ],
            resolve: {
                alias: {
                    "@": srcDir,
                },
            },
            css: {
                modules: {
                    localsConvention: "camelCaseOnly",
                    generateScopedName: "[local]__[hash:base64:8]",
                },
                preprocessorOptions: {
                    scss: {
                        loadPaths: [join(srcDir, "styles")],
                    },
                },
            },
            worker: {
                format: "es",
            },
        });
    },
};

export default config;
