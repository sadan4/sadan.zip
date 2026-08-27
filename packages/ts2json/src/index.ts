import { __String, CompilerHost, CompilerOptions, createCompilerHost, createProgram, displayPartsToString, ElementFlags, Extension, IndexKind, InternalSymbolName, isIdentifier, Program, resolveModuleName, SourceFile, Symbol, SymbolFlags, TupleTypeReference, Type, TypeChecker, TypeFlags } from "typescript";
import { AnySchema, SchemaBase, SchemaIntersection, SchemaObject, SchemaTuple, SchemaUnion } from "./schema";
import { createVirtualProgram, DEFAULT_COMPILER_OPTIONS } from "./program";

function error(msg: string): never { 
    throw new Error(msg);
}

function assert<T>(value: T, msg: string): asserts value { 
    if (!value) {  
        error(msg);
    }
}

/**
 * `obj[key] = value` sets the prototype instead of a key when `key` is `__proto__`,
 * which silently drops the property
 */
function setKey<T>(obj: Record<string, T>, key: string, value: T): void { 
    Object.defineProperty(obj, key, {
        value,
        writable: true,
        enumerable: true,
        configurable: true,
    });
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

/**
 * the extensions module resolution lands on when a module actually ships types
 *
 * anything else (`.js`, `.jsx`, `.mjs`, `.cjs`, `.json`) has no type information to read
 */
// `ResolvedModuleFull.extension` is typed as a plain string, not `Extension`
const TYPED_EXTENSIONS: ReadonlySet<string> = new Set<string>([
    Extension.Dts,
    Extension.Dmts,
    Extension.Dcts,
    Extension.Ts,
    Extension.Tsx,
    Extension.Mts,
    Extension.Cts,
]);

const BYTE_MAX = 255;

/**
 * a numeric index signature only allows keys that stringify to a number
 */
const NUMERIC_KEY_PATTERN = "^-?\\d+$";

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
    #program: Program;
    private constructor(program: Program, rootFile: SourceFile) { 
        this.#c = program.getTypeChecker();
        this.#program = program;
        this.#rootFile = rootFile;
    }

    public static createVirtual(tsCode: string): Analyzer { 
        const { program, rootFile } = createVirtualProgram(tsCode);
        return new Analyzer(program, rootFile);
    }

    public static createFromFile(filePath: string, options: CompilerOptions = DEFAULT_COMPILER_OPTIONS, host?: CompilerHost): Analyzer {
        const program = createProgram({
            options,
            rootNames: [filePath],
            host,
        });
        const rootFile = program.getSourceFile(filePath);
        assert(rootFile, `file ${filePath} not found in program`);
        return new Analyzer(program, rootFile);
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
        // resolution only lands on a runtime file when there are no types for the module
        assert(
            TYPED_EXTENSIONS.has(resolvedModule.extension),
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
     * `any` and `unknown` accept every value, so they constrain nothing
     *
     * every value includes `undefined` and `null`, but that does not make them
     * optional or nullable, it makes them unconstrained
     */
    #isUnconstrained(ty: Type): boolean { 
        return !!(ty.flags & (TypeFlags.Any | TypeFlags.Unknown));
    }

    /**
     * null is not considered optional
     */
    #isJsonOptional(ty: Type): boolean { 
        return !this.#isUnconstrained(ty) && this.#c.isTypeAssignableTo(this.#c.getUndefinedType(), ty);
    }

    /**
     * `null` is **NOT** nullable
     * A type is nullable if `null` is assignable to it **AND** it is not `null` itself
     */
    #isNullable(ty: Type): boolean {
        return !this.#isUnconstrained(ty)
            && this.#c.isTypeAssignableTo(this.#c.getNullType(), ty)
            && !(ty.flags & TypeFlags.Null);
    }

    /**
     * a function has no json representation, `JSON.stringify` drops the property entirely
     */
    #isCallable(ty: Type): boolean { 
        const nonNullable = this.#c.getNonNullableType(ty);
        return nonNullable.getCallSignatures().length > 0 || nonNullable.getConstructSignatures().length > 0;
    }

    /**
     * a member keyed by a symbol, eg: `[Symbol.toPrimitive]()`
     *
     * the checker names these `__@toPrimitive@620`, which is not a key any json object can have
     */
    #isSymbolKeyed(prop: Symbol): boolean { 
        // `__String` is branded to keep it apart from real identifiers, but it is a string
        return (prop.escapedName as string).startsWith("__@");
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
            if (this.#isSymbolKeyed(prop)) { 
                continue;
            }
            // a property can have multiple declarations
            // eg: interface Base { foo: string | number } interface Derived extends Base { foo: string }
            // any is valid, the symbols are unique between them
            const decl = prop.valueDeclaration ?? prop.declarations?.[0] ?? this.#rootFile;
            const ty = this.#c.getTypeOfSymbolAtLocation(prop, decl);
            // methods and function-valued properties never survive serialization
            if (this.#isCallable(ty)) { 
                continue;
            }
            const jsDocIR = prop.getDocumentationComment(this.#c);
            const jsDoc = displayPartsToString(jsDocIR);
            const name = prop.getName();
            if (!this.#isJsonOptional(ty)) {
                s.required.push(name);
            }
            const schema = this.#getSchemaForNullishType(ty);
            if (jsDocIR.length) {
                schema.description = jsDoc;
            }
            setKey(s.properties, name, schema);
            // @deprecated tag could have empty string value
            if (this.#isDeprecated(prop) != null) { 
                schema.deprecated = true;
            }
        }
        for (const { keyType, type: valueType } of this.#c.getIndexInfosOfType(type)) { 
            const schema = this.#getSchemaForNullishType(valueType);
            if (keyType.flags & TypeFlags.String) { 
                s.additionalProperties = schema;
            } else if (keyType.flags & TypeFlags.Number) { 
                s.patternProperties ??= {};
                s.patternProperties[NUMERIC_KEY_PATTERN] = schema;
            } else { 
                error(`index signature on ${this.#c.typeToString(type)} has an unsupported key type ${this.#c.typeToString(keyType)}`);
            }
        }
        if (!s.required.length) { 
            delete s.required;
        }
        return s;
    }

    /**
     * strips `undefined` and `null` off of `ty` before getting its schema
     *
     * a nullable type gets wrapped in an `anyOf` with `null`, an optional one does not,
     * because optionality is expressed by the container (`required` / `minItems`)
     */
    #getSchemaForNullishType(ty: Type): AnySchema { 
        const wasNullable = this.#isNullable(ty);
        if (wasNullable || this.#isJsonOptional(ty)) {
            ty = this.#c.getNonNullableType(ty);
        }
        const schema = this.getSchemaForType(ty);
        return wasNullable ? { anyOf: [schema, { type: "null" }] } : schema;
    }

    /**
     * json schema can only express a rest element at the end of a tuple,
     * so `[string, ...number[], boolean]` is an error
     */
    #getSchemaForTupleType(tyRef: TupleTypeReference): SchemaTuple { 
        const { elementFlags, labeledElementDeclarations } = tyRef.target;
        const elems = this.#c.getTypeArguments(tyRef);
        const s: SchemaTuple = {
            type: "array",
            prefixItems: [],
            items: false,
        };
        let minItems = 0;
        let sawNonRequired = false;
        for (let i = 0; i < elems.length; i++) { 
            const flags = elementFlags[i];
            const schema = this.#getSchemaForNullishType(elems[i]);
            // a tuple is either fully labeled or not labeled at all, but the array is sparse for rest params
            const label = labeledElementDeclarations?.[i]?.name;
            if (label && isIdentifier(label)) { 
                schema.description = label.text;
            }
            // Variable is Rest | Variadic
            if (flags & ElementFlags.Variable) { 
                assert(i === elems.length - 1, `tuple type ${this.#c.typeToString(tyRef)} has elements after its rest element`);
                s.items = schema;
                break;
            }
            if (flags & ElementFlags.Optional) { 
                sawNonRequired = true;
            } else { 
                assert(!sawNonRequired, `tuple type ${this.#c.typeToString(tyRef)} has a required element after an optional one`);
                minItems++;
            }
            s.prefixItems.push(schema);
        }
        if (minItems) { 
            s.minItems = minItems;
        }
        return s;
    }

    /**
     * types declared in the default lib that have no structural json representation,
     * eg: `RegExp`, whose properties are all methods
     *
     * returns undefined when `ty` is not one of them
     */
    #getSchemaForWellKnownType(ty: Type): AnySchema | undefined { 
        const sym = ty.getSymbol();
        const decl = sym?.declarations?.[0];
        if (!decl || !this.#program.isSourceFileDefaultLibrary(decl.getSourceFile())) { 
            return;
        }
        switch (sym.getName()) { 
            // json has no regex literal, only the source text of one
            case "RegExp": return { type: "string", format: "regex" };
            // json has no binary type, so bytes are the numbers `Array.from` would give
            case "Uint8Array": return { type: "array", items: { type: "number", minimum: 0, maximum: BYTE_MAX } };
            // every member of Date is a method, `toJSON` serializes it to an iso string
            case "Date": return { type: "string", format: "date-time" };
        }
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
        // must come first, because `any` and `never` are both assignable to `readonly any[]`
        if (this.#isUnconstrained(ty)) { 
            return {};
        }
        if (ty.flags & TypeFlags.Never) { 
            return { not: {} };
        }
        // must come before TypeFlags.Object, because arrays are objects
        if (this.#c.isTupleType(ty)) { 
            return this.#getSchemaForTupleType(ty as TupleTypeReference);
        }
        if (this.#c.isArrayLikeType(ty)) { 
            const elemType = this.#c.getIndexTypeOfType(ty, IndexKind.Number);
            assert(elemType, `Array-like type ${this.#c.typeToString(ty)} has no index type`);
            return { type: "array", items: this.#getSchemaForNullishType(elemType) };
        }
        if (ty.flags & TypeFlags.String) {
            return { type: "string" };
        }
        if (ty.flags & TypeFlags.Number) { 
            return { type: "number" };
        }
        if (ty.flags & TypeFlags.Object) {
            return this.#getSchemaForWellKnownType(ty) ?? this.#getSchemaForObjectType(ty);
        }
        if (ty.flags & TypeFlags.Null) { 
            return { type: "null" };
        }
        if (ty.isStringLiteral()) { 
            return { type: "string", const: ty.value };
        }
        // this also covers numeric enum members, which are number literals with an extra flag
        if (ty.isNumberLiteral()) { 
            return { type: "number", const: ty.value };
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
            // a symbol is only resolved at the root, nested types go through getSchemaForType
            return { $schema: this.$schema, ...this.#getSchemaForInterface(sym) };
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