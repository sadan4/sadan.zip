import { createAnalyzerFromModule } from "..";

// eslint-disable-next-line unused-imports/no-unused-imports
import type { BuildOptions as _BuildOptions } from "rolldown";
import type { __String } from "typescript";
import { expect, it } from "vitest";

it("rolldown.d.ts", () => {
    const analyzer = createAnalyzerFromModule("rolldown");
    const schema = analyzer.getSymbolForExportName("BuildOptions" as __String);

    if (!schema) {
        throw new Error("BuildOptions not found");
    }

    const out = analyzer.getSchemaForSymbol(schema);

    expect(out).toMatchSnapshot();
});
