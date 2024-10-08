import * as SJsPrimitives from "ext:sjs_primitives/src/js/index.ts"
import { insertRow, searchRows } from "ext:sjs_engine/src/js/ops.ts";
import { QueryBuilder } from "ext:sjs_engine/src/js/query.ts";
const core = globalThis.Deno.core;
class SchemeJS {

    static get Table() {
        return SJsPrimitives.Table;
    }

    static get Column() {
        return SJsPrimitives.Column;
    }

    static get DataTypes() {
        return SJsPrimitives.DataTypes;
    }

    static get QueryBuilder() {
        return QueryBuilder;
    }

    static get rawInsert() {
        return insertRow;
    }

    static get insert() {
        return (...data) => {
            if(!globalThis.SJS_CONTEXT) {
                throw new Error("SJS_CONTEXT is necessary when using a `insert`. Consider using `rawInsert` otherwise.");
            } else {
                let { dbName, tblName } = globalThis.SJS_CONTEXT;

                tblName = data.length === 2 ? data[0] : tblName;

                if(!dbName) {
                    throw new Error("SchemeJS.insert requires a database");
                } else if(!tblName) {
                    throw new Error("SchemeJS.insert requires a table. `SchemeJS.insert(table_name, row)`");
                }

                return insertRow(dbName, tblName, data.length === 2 ? data[1] : data[0]);
            }
        }
    }

    static get query() {
        return (q: QueryBuilder) => {
            if(!(q instanceof QueryBuilder)) {
                throw new Error("Queries must be performed with SchemeJS.QueryBuilder");
            } else {
                return searchRows(q.dbName, q.tableName, q.build())
            }
        }
    }

    static print(msg: string) {
        core.ops.sjs_op_print(msg);
    }

}

export const SJSGlobal = {
    SchemeJS
}