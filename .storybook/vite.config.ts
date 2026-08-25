import { defineConfig } from "vite";

// pointed to by `viteConfigPath` in main.ts so the builder doesn't fall back
// to the root vite.config.ts, whose TanStack Start/Cloudflare/omt plugins
// aren't meant for the storybook build
export default defineConfig({});
