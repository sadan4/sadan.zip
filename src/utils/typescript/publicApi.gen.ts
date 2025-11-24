import { assert } from "../error";
import { dedent } from "../string";

import { join } from "node:path";
import type { GeneratorArgs } from "rollup-plugin-generate";
import { createProgram, type Program, type SourceFile, type Symbol, type Type, type TypeChecker } from "typescript";

interface NodeEntry {
    name: string;
    props: Set<string>;
}

class DTSAnalyzer {
    private program: Program;
    private tc: TypeChecker;
    private file: SourceFile;
    private fileSym: Symbol;
    private fileExports: Symbol[];
    private skExport: Symbol;
    private syntaxKinds: ReadonlySet<Type>;
    private _syntaxKindNameMap: WeakMap<Type, string> = new WeakMap();

    constructor(private readonly path: string, private d: (msg: string) => void = () => {}) {
        this.program = createProgram([this.path], { strictNullChecks: true });
        this.tc = this.program.getTypeChecker();

        const file = this.program.getSourceFile(this.path);

        assert(file);
        this.file = file;

        const fileSym = this.tc.getSymbolAtLocation(this.file);

        assert(fileSym);

        this.fileSym = fileSym;
        this.fileExports = this.tc.getExportsOfModule(this.fileSym);

        const skExport = this.fileExports.find((x) => x.getName() === "SyntaxKind");

        assert(skExport);
        this.skExport = skExport;

        const syntaxKinds = new Set(this.tc.getTypeOfSymbol(this.skExport).getProperties()
            .map((prop) => this.tc.getDeclaredTypeOfSymbol(prop)));

        this.syntaxKinds = syntaxKinds;
    }

    private getPropertiesFromSymbol(sym: Symbol): Symbol[] {
        return this.tc.getDeclaredTypeOfSymbol(sym).getProperties();
    }

    private tryGetSyntaxKindOfNode(node: Symbol): string[] | undefined {
        const nodeType = this.tc.getDeclaredTypeOfSymbol(node);
        const kind = nodeType.getProperty("kind");

        if (kind == null) {
            return;
        }

        const [decl] = kind.getDeclarations() ?? [];

        if (decl == null) {
            return;
        }

        const kindType = this.tc.getTypeOfSymbolAtLocation(kind, decl);
        const kinds = this.intersectWithSyntaxKind(kindType);

        if (kinds.length) {
            return kinds;
        }
    }

    private getSyntaxKindName(type: Type): string {
        if (this._syntaxKindNameMap.has(type)) {
            return this._syntaxKindNameMap.get(type)!;
        }

        const name = this.tc.typeToString(type, undefined);

        return name.replace(/^SyntaxKind\./, "");
    }

    private intersectWithSyntaxKind(type: Type): string[] {
        const ret: string[] = [];
        const base = this.tc.getBaseConstraintOfType(type) ?? type;

        for (const sk of this.syntaxKinds) {
            if (this.tc.isTypeAssignableTo(sk, base)) {
                ret.push(this.getSyntaxKindName(sk));
            }
        }

        return ret;
    }

    private getPropertiesForNodeType(nodeSym: Symbol): string[] {
        const props = this.getPropertiesFromSymbol(nodeSym);

        return props
            .map((p) => p.getName())
            .filter((p) => !p.startsWith("_"));
    }

    generate(): NodeEntry[] {
        const nodes: Map<string, Set<string>> = new Map();

        for (const sym of this.fileExports) {
            const kinds = this.tryGetSyntaxKindOfNode(sym);

            if (!kinds) {
                continue;
            }

            const props = new Set(this.getPropertiesForNodeType(sym));

            for (const kind of kinds) {
                nodes.set(kind, (nodes.get(kind) ?? new Set()).union(props));
            }
        }

        return Array
            .from(nodes.entries())
            .map(([name, props]) => ({
                name,
                props,
            } satisfies NodeEntry));
    }

    debug() {
        const testThing = this.fileExports.find((x) => x.getName() === "KeywordToken");

        this.d(`testThing: ${testThing}`);
        if (!testThing)
            return;

        const matches = this.tryGetSyntaxKindOfNode(testThing);

        this.d(`matches: ${matches?.join(", ")}`);
        this.d = () => {};
    }
}

export function generate(_: GeneratorArgs) {
    const path = join(process.cwd(), "node_modules", "typescript", "lib", "typescript.d.ts");
    const analyzer = new DTSAnalyzer(path, _.info);
    const nodes: NodeEntry[] = analyzer.generate();

    return dedent`
        // This file is generated. Do not edit.

        type PublicNodeProperties = Readonly<Map<string, Readonly<Set<string>>>>;

        export const publicNodeProperties: PublicNodeProperties = Object.freeze(new Map([
            ${nodes
                /* eslint-disable @stylistic/indent */
                .map(({ name, props }) => {
                    return `[
                        ${JSON.stringify(name)},
                        new Set([
                            ${
                                Array.from(props)
                                    .map((p) => JSON.stringify(p))
                                    .join(",\n".padEnd(4 * 2))
                            }
                        ])
                    ]`;
                })
                .join(",\n".padEnd(4 * 1))
                /* eslint-enable @stylistic/indent */
            }
        ]));
    `;
}
