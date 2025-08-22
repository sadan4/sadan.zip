import viteConfig from "./vite.config";

import type { UserConfig } from "vite";
import { defineConfig, mergeConfig } from "vitest/config";

function removeLogFromViteConfig(viteConfig: UserConfig): UserConfig {
    viteConfig.plugins = viteConfig.plugins?.filter((plugin: any) => plugin.name !== "log-server-requests");
    return viteConfig;
}

export default mergeConfig(
    removeLogFromViteConfig(viteConfig),
    defineConfig({
        test: {
            browser: {
                enabled: true,
                headless: true,
                provider: "playwright",
                instances: [
                    {
                        browser: "chromium",
                    },
                ],

            },
            includeTaskLocation: true,
            globals: true,
            setupFiles: ["./src/test/setup.ts"],
            exclude: ["**/node_modules/**", "**/dist/**", "**/.idea/**", "**/.git/**", "**/.cache/**"],
        },
    }),
);
