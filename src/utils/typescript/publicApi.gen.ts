import { assert } from "../error";
import { dedent } from "../string";

import { join } from "node:path";
import type { GeneratorArgs } from "rollup-plugin-generate";
import { type __String, createProgram, type Program, type SourceFile, type Symbol, type TypeChecker } from "typescript";

class DTSAnalyzer {
    private program: Program;
    private tc: TypeChecker;
    private file: SourceFile;
    private fileSym: Symbol;
    private fileExports: Symbol[];

    constructor(private readonly path: string) {
        this.program = createProgram([this.path], { strictNullChecks: true });
        this.tc = this.program.getTypeChecker();

        const file = this.program.getSourceFile(this.path);

        assert(file);
        this.file = file;

        const fileSym = this.tc.getSymbolAtLocation(this.file);

        assert(fileSym);

        this.fileSym = fileSym;
        this.fileExports = this.tc.getExportsOfModule(this.fileSym);
    }

    private getPropertiesFromSymbol(sym: Symbol): Symbol[] {
        return this.tc.getDeclaredTypeOfSymbol(sym).getProperties();
    }

    private tryGetSyntaxKindOfNode(node: Symbol): string | undefined {
        const kind = node.members?.get("kind" as __String);

        if (kind == null) {
            return;
        }

        const [decl] = kind.getDeclarations() ?? [];

        if (decl == null) {
            return;
        }

        const kindType = this.tc.getTypeOfSymbolAtLocation(kind, decl);

        if (kindType.isUnionOrIntersection()) {
            return;
        }

        const kindTypeString = this.tc.typeToString(kindType);

        if (!kindTypeString.startsWith("SyntaxKind.")) {
            return;
        }

        return kindTypeString.replace("SyntaxKind.", "");
    }

    getPropertiesForNodeType(nodeSym: Symbol): string[] {
        const props = this.getPropertiesFromSymbol(nodeSym);

        return props
            .map((p) => p.getName())
            .filter((p) => !p.startsWith("_"));
    }

    *getAllNodeTypes(): Generator<[nodeSym: Symbol, syntaxKindName: string]> {
        for (const sym of this.fileExports) {
            const name = this.tryGetSyntaxKindOfNode(sym);

            if (name) {
                yield [sym, name];
            }
        }
    }
}

export function generate(_: GeneratorArgs) {
    const path = join(process.cwd(), "node_modules", "typescript", "lib", "typescript.d.ts");
    const analyzer = new DTSAnalyzer(path);

    interface NodeEntry {
        name: string;
        props: string[];
    }

    const nodes: NodeEntry[] = [];

    for (const [nodeSym, syntaxKindName] of analyzer.getAllNodeTypes()) {
        const props = analyzer.getPropertiesForNodeType(nodeSym);

        nodes.push({
            name: JSON.stringify(syntaxKindName),
            props: props.map((p) => JSON.stringify(p)),
        });
    }

    return dedent`
        // This file is generated. Do not edit.

        type PublicNodeProperties = ReadonlyMap<string, ReadonlySet<string>>;

        export const publicNodeProperties: PublicNodeProperties = Object.freeze(new Map([
            ${nodes
                /* eslint-disable @stylistic/indent */
                .map(({ name, props }) => {
                    return `[
                        ${name},
                        new Set([
                            ${props.join(",\n".padEnd(4 * 2))}
                        ])
                    ]`;
                })
                .join(",\n".padEnd(4 * 1))
                /* eslint-enable @stylistic/indent */
            }
        ]));
    `;
}
