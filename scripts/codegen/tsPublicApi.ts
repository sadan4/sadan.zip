import { assert } from "@/utils/error";
import { dedent } from "@/utils/string";

import { mkdir, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { createProgram, type Program, type SourceFile, type Symbol, SyntaxKind, type Type, type TypeChecker } from "typescript";

interface NodeEntry {
    name: string;
    props: Set<string>;
}

class DTSAnalyzer {
    private readonly program: Program;
    private readonly tc: TypeChecker;
    private readonly file: SourceFile;
    private readonly fileSym: Symbol;
    private readonly fileExports: Symbol[];
    private readonly skExport: Symbol;
    private readonly syntaxKinds: ReadonlySet<Type>;
    private readonly _syntaxKindNameMap = new WeakMap<Type, string>();

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

        const [decl = null] = kind.getDeclarations() ?? [];

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

    private _markerMap: Map<string, string> | null = null;

    getMarkerMap(): Map<string, string> {
        if (this._markerMap) {
            return this._markerMap;
        }

        const ret = new Map<string, string>();
        const firstOrLastMarkers = Object.keys(SyntaxKind).filter((k) => k.startsWith("First") || k.startsWith("Last"));

        for (const marker of firstOrLastMarkers) {
            for (const key in SyntaxKind) {
                if (SyntaxKind[key as keyof typeof SyntaxKind] === SyntaxKind[marker as keyof typeof SyntaxKind]) {
                    ret.set(marker, key);
                    break;
                }
            }
        }

        return (this._markerMap = ret);
    }

    generate(): NodeEntry[] {
        const nodes = new Map<string, Set<string>>();

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

        for (const [marker, resolved] of this.getMarkerMap()) {
            if (nodes.has(resolved)) {
                nodes.set(marker, nodes.get(resolved)!);
            }
        }

        return Array
            .from(nodes.entries())
            .map(([name, props]) => ({
                name,
                props,
            } satisfies NodeEntry));
    }
}

const path = join(process.cwd(), "node_modules", "typescript", "lib", "typescript.d.ts");
const analyzer = new DTSAnalyzer(path);
const nodes: NodeEntry[] = analyzer.generate();
const rootPath = resolve(import.meta.dirname, "..", "..");
const genDir = resolve(rootPath, "src", "utils", "typescript", "generated");

await mkdir(genDir, { recursive: true });

const content = dedent`
    /* eslint-disable */
    // This file is generated by scripts/codegen/tsPublicApi.ts. Do not edit.

    type PublicNodeProperties = ReadonlyMap<string, ReadonlySet<string>>;

    export const publicNodeProperties: PublicNodeProperties = Object.freeze(new Map([
        ${nodes
            /* eslint-disable @stylistic/indent */
            .map(({ name, props }) => {
                return dedent`[
                    ${JSON.stringify(name)},
                    new Set([
                        ${
                            Array.from(props)
                                .map((p) => JSON.stringify(p))
                                .join(",\n")
                        }
                    ])
                ]`;
            })
            .join(",\n")
            /* eslint-enable @stylistic/indent */
        }
    ]));
`;

const publicApiPath = resolve(genDir, "publicApi.ts");

await writeFile(publicApiPath, `${content}\n`);

const markerMapContent = dedent`
    /* eslint-disable */
    // This file is generated by scripts/codegen/tsPublicApi.ts. Do not edit.

    export const markerMap: ReadonlyMap<string, string> = Object.freeze(new Map(${JSON.stringify(Array.from(analyzer.getMarkerMap()))}));
`;

const markerMapPath = resolve(genDir, "markerMap.ts");

await writeFile(markerMapPath, `${markerMapContent}\n`);
