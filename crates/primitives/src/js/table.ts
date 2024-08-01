import { Column } from "ext:sjs_primitives/src/js/column.ts";

export class Table {
    public name: string;
    public columns: Record<string, Column> = {};

    constructor(name: string) {
        this.name = name;
    }

    addColumn(col: Column) {
        this.columns[col.name] = col;
        return this;
    }
}