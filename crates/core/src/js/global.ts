import * as SJsPrimitives from "ext:sjs_primitives/src/js/index.ts"
import { insertRow } from "ext:sjs_engine/src/js/ops.ts";
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

    static get insert() {
        return insertRow;
    }

    static print(msg: string) {
        core.print(msg);
    }

}

export const SJSGlobal = {
    SchemeJS
}