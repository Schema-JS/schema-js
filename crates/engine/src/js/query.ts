type DataValue = {
    String: string,
} | { Uuid: Uuid } | "Null" | {
    Boolean: boolean,
} | {
    Number: number
}

interface QueryVal {
    key: string;
    filter_type: string;
    value: DataValue;
}

interface Condition {
    Condition: QueryVal;
}

interface And {
    And: QueryOps[];
}

interface Or {
    Or: QueryOps[];
}

type QueryOps = Condition | And | Or;

export class Uuid {
    private value: string;
    constructor(value: string) {
        this.value = value;
    }
}

const parseType = (val: any): DataValue => {
    if(typeof val === 'string') {
        return {
            String: val
        }
    } else if(typeof val === 'number') {
        return {
            Number: val
        }
    } else if (typeof val === 'boolean') {
        return {
            Boolean: val
        }
    } else if(typeof val === 'object' && val instanceof Uuid) {
        return {
            Uuid: val
        }
    } else if(val === null) {
        return "Null"
    } else {
        throw new Error("Invalid Data Type")
    }
}

export class QueryBuilder {
    private query: QueryOps[] = [];
    public readonly dbName: string
    public readonly tableName: string;

    constructor(dbName?: string, tableName?: string) {
        let ctx = globalThis.SJS_CONTEXT;

        this.dbName = dbName || ctx?.dbName;
        this.tableName = tableName || ctx?.tblName;
    }

    // Static methods for convenience
    static where(dbName: string, tableName: string, key: string, filter_type: string, value: any) {
        const builder = new QueryBuilder(dbName, tableName);
        return builder.where(key, filter_type, value);
    }

    static and(dbName: string, tableName: string, callback: (builder: QueryBuilder) => void) {
        const builder = new QueryBuilder(dbName, tableName);
        return builder.and(callback);
    }

    static or(dbName: string, tableName: string, callback: (builder: QueryBuilder) => void) {
        const builder = new QueryBuilder(dbName, tableName);
        return builder.or(callback);
    }

    // Method to add a basic condition
    where(key: string, filter_type: string, value: any) {
        this.query.push({
            Condition: {
                key,
                filter_type,
                value: parseType(value)
            }
        });
        return this;
    }

    // Method to add an AND condition
    and(callback: (builder: QueryBuilder) => void) {
        const builder = new QueryBuilder(this.dbName, this.tableName);
        callback(builder);
        this.query.push({
            And: builder.build(false)
        });
        return this;
    }

    // Method to add an OR condition
    or(callback: (builder: QueryBuilder) => void) {
        const builder = new QueryBuilder(this.dbName, this.tableName);
        callback(builder);
        this.query.push({
            Or: builder.build(false)
        });
        return this;
    }

    // Build the final query structure
    build(notFinal?: boolean) {
        const query = notFinal === false ? this.query : this.query[0];
        return query;
    }
}