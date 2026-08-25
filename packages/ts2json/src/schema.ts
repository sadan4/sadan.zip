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

export type AnySchema = SchemaIntersection | SchemaUnion | SchemaString | SchemaNumber | SchemaObject | SchemaArray | SchemaBoolean | SchemaNull;

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
     * the constant value of the string
     * 
     * if this is present, then the type is *only* this value
     */
    const?: string;
}

export interface SchemaNumber extends SchemaBase { 
    type: "number";
}

export interface SchemaObject extends SchemaBase { 
    type: "object";
    properties: Record<string, AnySchema>;
    required?: string[];
    additionalProperties?: boolean;
}

export interface SchemaArray extends SchemaBase { 
    type: "array";
    items: AnySchema;
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
