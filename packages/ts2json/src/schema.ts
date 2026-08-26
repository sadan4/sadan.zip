import { Analyzer } from ".";

export interface SchemaBase { 
    /**
     * only present on the root
     */
    $schema?: Analyzer["$schema"];
    type?: "string" | "number" | "object" | "array" | "boolean" | "null";
    /**
     * a description of the type/property/value
     */
    description?: string;
    /**
     * indicates that the type/property/value is deprecated
     */
    deprecated?: boolean;
}

export type AnySchema = SchemaIntersection | SchemaUnion | SchemaString | SchemaNumber | SchemaObject | SchemaArray | SchemaTuple | SchemaBoolean | SchemaNull;

export interface SchemaIntersection extends SchemaBase {
    type?: undefined;
    allOf: AnySchema[];
}

export interface SchemaUnion extends SchemaBase { 
    type?: undefined;
    anyOf: AnySchema[];
}

export interface SchemaString extends SchemaBase { 
    type: "string";
    /**
     * a semantic constraint on the string, eg: a `RegExp` is a `"regex"`
     */
    format?: "regex";
    /**
     * the constant value of the string
     * 
     * if this is present, then the type is *only* this value
     */
    const?: string;
}

export interface SchemaNumber extends SchemaBase { 
    type: "number";
    /**
     * the smallest allowed value, inclusive
     */
    minimum?: number;
    /**
     * the largest allowed value, inclusive
     */
    maximum?: number;
}

export interface SchemaObject extends SchemaBase { 
    type: "object";
    properties: Record<string, AnySchema>;
    required?: string[];
    /**
     * the schema for properties whose name matches the regex, keyed by that regex
     *
     * comes from a numeric index signature, because json object keys are always strings
     */
    patternProperties?: Record<string, AnySchema>;
    /**
     * `false` forbids properties not listed above, a schema constrains them
     *
     * comes from a string index signature
     */
    additionalProperties?: AnySchema | boolean;
}

export interface SchemaArray extends SchemaBase { 
    type: "array";
    items: AnySchema;
}

/**
 * a fixed length (or fixed prefix) array
 */
export interface SchemaTuple extends SchemaBase { 
    type: "array";
    /**
     * the schema of each element, by position
     */
    prefixItems: AnySchema[];
    /**
     * the schema of the elements after `prefixItems`
     *
     * `false` means there are none, ie: the tuple has no rest element
     */
    items: AnySchema | false;
    /**
     * the number of leading elements that are required
     *
     * omitted when zero
     */
    minItems?: number;
}

export interface SchemaBoolean extends SchemaBase { 
    type: "boolean";
    /**
     * the constant value of the boolean
     * 
     * if this is present, then the type is *only* this value
     */
    const?: boolean;
}

export interface SchemaNull extends SchemaBase { 
    type: "null";
}
