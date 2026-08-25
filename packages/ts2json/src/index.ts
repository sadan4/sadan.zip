import { __String, CompilerHost, CompilerOptions, createCompilerHost, createProgram, displayPartsToString, Extension, IndexKind, InternalSymbolName, Program, resolveModuleName, SourceFile, Symbol, SymbolFlags, TupleType, TupleTypeReference, Type, TypeChecker, TypeFlags } from "typescript";
import { AnySchema, SchemaBase, SchemaIntersection, SchemaObject, SchemaUnion } from "./schema";
import { createVirtualProgram, DEFAULT_COMPILER_OPTIONS } from "./program";
import { popcnt } from "./utils";

function error(msg: string): never { 
    throw new Error(msg);
}

function assert<T>(value: T, msg: string): asserts value { 
    if (!value) {  
        error(msg);
    }
}

/**
 * enum at runtime
 */
type RuntimeEnum = Record<string, string | number>;

interface EnumFlag {
    name: string;
    /**
     * power of 2
     */
    value: number;
}

function extractFlags<E extends RuntimeEnum>(enumObj: E): EnumFlag[] {
    const flags: EnumFlag[] = [];
    for (const [key, value] of Object.entries(enumObj)) {
        // reverse mappings are strings, and only single-bit members are flags
        if (typeof value !== "number" || (value & (value - 1))) {
            continue;
        }
        flags.push({ name: key, value });
    }
    return flags;
}

function makeFlagHumanizer<E extends RuntimeEnum>(enumObj: E): (v: number) => string {
    const flags = extractFlags(enumObj);
    return function humanizeFlags(v: number): string {
        const names: string[] = [];
        for (const { name, value} of flags) {
            if (v & value) {
                names.push(name);
            }
        }
        return names.length ? names.join(" | ") : "0";
    }
}

const humanizeTypeFlags = makeFlagHumanizer(TypeFlags);
const humanizeSymbolFlags = makeFlagHumanizer(SymbolFlags);

export class Analyzer {
    /**
     * the version of the JSON schema that is emitted
     */
    public get $schema() { 
        return "https://json-schema.org/draft/2020-12/schema" as const;
    }
    // #tsCode: string;
    // #host: CompilerHost;
    #c: TypeChecker;
    #rootFile: SourceFile;
    // #program: Program;
    private constructor(c: TypeChecker, rootFile: SourceFile) { 
        this.#c = c;
        this.#rootFile = rootFile;
    }

    public static createVirtual(tsCode: string): Analyzer { 
        const { program, rootFile } = createVirtualProgram(tsCode);
        return new Analyzer(program.getTypeChecker(), rootFile);
    }

    public static createFromFile(filePath: string, options: CompilerOptions = DEFAULT_COMPILER_OPTIONS, host?: CompilerHost): Analyzer {
        const program = createProgram({
            options,
            rootNames: [filePath],
            host,
        });
        const rootFile = program.getSourceFile(filePath);
        assert(rootFile, `file ${filePath} not found in program`);
        return new Analyzer(program.getTypeChecker(), rootFile);
    }

    /**
     * create an analyzer from a module name, eg: "esbuild" or "@foo/bar"
     *
     * @param containingFile the file the module is resolved relative to.
     * defaults to a fake file in the current working directory
     */
    public static createFromModule(moduleName: string, containingFile?: string, options: CompilerOptions = DEFAULT_COMPILER_OPTIONS): Analyzer {
        const host = createCompilerHost(options, true);
        // the file doesn't need to exist, only its directory is used to walk up looking for node_modules
        const from = containingFile ?? `${host.getCurrentDirectory()}/__ts2json__.ts`;
        const { resolvedModule } = resolveModuleName(moduleName, from, options, host);
        assert(resolvedModule, `could not resolve module ${moduleName} from ${from}`);
        // resolution only finds js when there are no types for the module
        assert(
            resolvedModule.extension !== Extension.Js && resolvedModule.extension !== Extension.Jsx,
            `module ${moduleName} resolved to ${resolvedModule.resolvedFileName}, which has no type declarations`
        );
        return Analyzer.createFromFile(resolvedModule.resolvedFileName, options, host);
    }

    public getSymbolForExportName(exportName: __String): Symbol | undefined { 
        return this.#c.getSymbolAtLocation(this.#rootFile)?.exports?.get(exportName);
    }

    #getSchemaForInterface(sym: Symbol): AnySchema {
        assert(sym.flags & SymbolFlags.Interface, `Symbol ${sym.getName()} is not an interface`);
        const type = this.#c.getDeclaredTypeOfSymbol(sym);
        assert(type.isClassOrInterface(), `Symbol ${sym.getName()} is not a class or interface`);
        return this.#getSchemaForObjectType(type);
    }

    /**
     * null is not considered optional
     */
    #isJsonOptional(ty: Type): boolean { 
        return this.#c.isTypeAssignableTo(this.#c.getUndefinedType(), ty);
    }

    /**
     * `null` is **NOT** nullable
     * A type is nullable if `null` is assignable to it **AND** it is not `null` itself
     */
    #isNullable(ty: Type): boolean {
        return this.#c.isTypeAssignableTo(this.#c.getNullType(), ty) && !(ty.flags & TypeFlags.Null);
    }

    /**
     * returns the text of the `\@deprecated` tag if present, otherwise undefined
     */
    #isDeprecated(sym: Symbol): string | undefined { 
        const tags = sym.getJsDocTags(this.#c);
        const deprecatedTag = tags.find(t => t.name === "deprecated");
        if (deprecatedTag) { 
            return displayPartsToString(deprecatedTag.text);
        }
    }

    #getSchemaForObjectType(type: Type): SchemaObject {
        const s: SchemaObject = {
            type: "object",
            properties: {},
            additionalProperties: false,
        };
        // typescript can't do inference if we declare it in the obj literal
        s.required = [];
        for (const prop of this.#c.getPropertiesOfType(type)) {
            const jsDocIR = prop.getDocumentationComment(this.#c);
            const jsDoc = displayPartsToString(jsDocIR);
            // a property can have multiple declarations
            // eg: interface Base { foo: string | number } interface Derived extends Base { foo: string }
            // any is valid, the symbols are unique between them
            const decl = prop.valueDeclaration ?? prop.declarations?.[0] ?? this.#rootFile;
            const name = prop.getName();
            let ty = this.#c.getTypeOfSymbolAtLocation(prop, decl);
            const isOptional = this.#isJsonOptional(ty);
            if (!isOptional) {
                s.required.push(name);
            }
            const wasNullable = this.#isNullable(ty);
            if (wasNullable || isOptional) {
                ty = this.#c.getNonNullableType(ty);
            }
            let schema = this.getSchemaForType(ty);
            if (wasNullable) { 
                schema = { anyOf: [schema, { type: "null" }] };
            }
            if (jsDocIR.length) {
                schema.description = jsDoc;
            }
            s.properties[name] = schema;
            // @deprecated tag could have empty string value
            if (this.#isDeprecated(prop) != null) { 
                schema.deprecated = true;
            }
        }
        if (!s.required.length) { 
            delete s.required;
        }
        return s;
    }

    #booleanLiteralValue(ty: Type): boolean { 
        const strRepr = this.#c.typeToString(ty);
        assert(ty.flags & TypeFlags.BooleanLiteral, `Type ${strRepr} is not a boolean literal`);
        switch (strRepr) { 
            case "true": return true;
            case "false": return false;
            default: error(`expected boolean literal type to be "true" or "false", got ${JSON.stringify(strRepr)}`);
        }
    }

    /**
     * handles primitives and objects
     */
    public getSchemaForType(ty: Type): AnySchema {
        // must come before TypeFlags.Object, because arrays are objects
        if (this.#c.isTupleType(ty)) { 
            const tyRef = ty as TupleTypeReference;
            const target = tyRef.target;
            const elems = this.#c.getTypeArguments(tyRef);
            for (let i = 0; i < elems.length; i++) { 
                const flags = target.elementFlags[i];
            }
            error("TODO: implement getSchemaForType for tuple types");
        }
        if (this.#c.isArrayLikeType(ty)) { 
            const elemType = this.#c.getIndexTypeOfType(ty, IndexKind.Number);
            assert(elemType, `Array-like type ${this.#c.typeToString(ty)} has no index type`);
            return { type: "array", items: this.getSchemaForType(elemType) };
        }
        if (ty.flags & TypeFlags.String) {
            return { type: "string" };
        }
        if (ty.flags & TypeFlags.Number) { 
            return { type: "number" };
        }
        if (ty.flags & TypeFlags.Object) {
            return this.#getSchemaForObjectType(ty);
        }
        if (ty.flags & TypeFlags.Null) { 
            return { type: "null" };
        }
        if (ty.isStringLiteral()) { 
            return { type: "string", const: ty.value };
        }
        if (ty.flags & TypeFlags.Boolean) {
            assert(!(ty.flags & TypeFlags.BooleanLiteral), `Type ${this.#c.typeToString(ty)} is a boolean literal, not a boolean`);
            return { type: "boolean" };
        }
        if (ty.flags & TypeFlags.BooleanLiteral) { 
            const val = this.#booleanLiteralValue(ty);
            return { type: "boolean", const: val };
        }
        if (ty.isUnion()) { 
            const s: SchemaUnion = { anyOf: [] };
            for (const t of ty.types) { 
                s.anyOf.push(this.getSchemaForType(t));
            }
            return s;
        }
        if (ty.isIntersection()) { 
            const s: SchemaIntersection = { allOf: [] };
            for (const t of ty.types) { 
                const schema = this.getSchemaForType(t);
                // remove additionalProperties if it's an object
                if (schema.type === "object") {
                    delete schema.additionalProperties;
                }
                s.allOf.push(schema);
            }
            return s;
        }
        error(`TODO: implement getSchemaForType for ${this.#c.typeToString(ty)} with flags ${humanizeTypeFlags(ty.flags)}`);
    }

    public getSchemaForSymbol(sym: Symbol): AnySchema { 
        if (sym.flags & SymbolFlags.Variable) { 
            error(`Symbol ${sym.getName()} is a variable, not an type`);
        }
        if (sym.flags & SymbolFlags.Interface) { 
            return this.#getSchemaForInterface(sym);
        }
        error(`TODO: implement getSchemaForSymbol for ${sym.getName()} with flags ${humanizeSymbolFlags(sym.flags)}`);
    }
}

export function handleDefaultExport(tsCode: string): SchemaBase { 
    const analyzer = Analyzer.createVirtual(tsCode);
    const defaultExportSym = analyzer.getSymbolForExportName(InternalSymbolName.Default);
    assert(defaultExportSym, "No default export found");
    return analyzer.getSchemaForSymbol(defaultExportSym);
}