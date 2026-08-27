import { defineConfig, type ViteUserConfig } from "vitest/config";

// annotated because isolatedDeclarations cannot infer a default export
const config: ViteUserConfig = defineConfig({
    test: {
        include: ["src/**/*.test.ts"],
        environment: "node",
    },
});

export default config;
