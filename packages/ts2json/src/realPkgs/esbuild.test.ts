import { expect, it } from "vitest";
import { Analyzer } from "..";
import { __String } from "typescript";
import { BuildOptions as _BuildOptions } from "esbuild";

it.todo("esbuild.d.ts", () => { 
    const analyzer = Analyzer.createFromModule("esbuild");
    const schema = analyzer.getSymbolForExportName("BuildOptions" as __String);
    if (!schema) throw new Error("BuildOptions not found");
    const jsonSchema = analyzer.getSchemaForSymbol(schema);
    expect(jsonSchema).toMatchInlineSnapshot();
})