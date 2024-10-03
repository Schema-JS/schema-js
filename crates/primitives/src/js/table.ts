import { Column } from "ext:sjs_primitives/src/js/column.ts";
import { Helper, HelperType } from "ext:sjs_helpers/src/js/helper.ts";

export class Table {
    public name: string;
    public columns: Record<string, Column> = {};
    public indexes = [];
    public primary_key = "_uid";
    public helpers: Helper[] = [];

    constructor(name: string) {
        this.name = name;
    }

    addColumn(col: Column) {
        this.columns[col.name] = col;
        return this;
    }

    addQuery(name: string, cb: any) {
        this.helpers.push(new Helper(name, HelperType.CustomQuery, cb));
        return this;
    }

    on(type: string, cb: any) {
        let lowerCaseType = type.toLowerCase();
        switch (lowerCaseType) {
            case "insert": {
                this.helpers.push(new Helper("default", HelperType.InsertHook, cb));
            }
            break;
            default: {
                throw new Error("Unknown hook type")
            }
        }
        return this;
    }
}