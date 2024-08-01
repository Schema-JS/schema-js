import * as SJsPrimitives from "ext:sjs_primitives/src/js/index.ts"

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

}

export const SJSGlobal = {
    SchemeJS
}