import { type ConfigExport, defineConfig } from "rolldown";
import { dts } from "rolldown-plugin-dts";

// annotated because isolatedDeclarations cannot infer a default export
const config: ConfigExport = defineConfig({
    input: "src/index.ts",
    platform: "neutral",
    external: ["typescript"],
    plugins: [
        dts({
            sourcemap: true,
        }),
    ],
    output: {
        dir: "dist",
        format: "esm",
        sourcemap: true,
    },
});

export default config;
