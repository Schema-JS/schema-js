import * as SJsPrimitives from "ext:sjs_primitives/src/js/index.ts"
import { insertRow } from "ext:sjs_engine/src/js/ops.ts";
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

}

export const SJSGlobal = {
    SchemeJS
}