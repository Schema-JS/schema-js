import { DataTypes } from "ext:sjs_primitives/src/js/dataTypes.ts";

export class Column {
    public name: string;
    public dataType: DataTypes;
    public defaultValue?: string;
    public comment?: string;

    constructor(name: string, dataType?: DataTypes) {
        this.name = name;
        this.dataType = dataType || DataTypes.String;
    }

    string() {
        this.dataType = DataTypes.String;
        return this;
    }

    boolean() {
        this.dataType = DataTypes.Boolean;
        return this;
    }

    withComment(comment: string) {
        this.comment_ = comment;
        return this;
    }

    withDefaultValue(val: any) {
        const mapping = {
            [DataTypes.String]: {
                type: 'string',
                validator: (x:any) => typeof x === 'string'
            },
            [DataTypes.Boolean]: {
                type: 'boolean',
                validator: (x:any) => typeof x === 'boolean'
            }
        };

        const mapEntry = mapping[this.dataType];
        if (mapEntry && mapEntry.validator && !mapEntry.validator(val)) {
            throw new Error(`Default value does not match column type. ${this.name} is of type '${mapEntry.type}'.`);
        }

        this.defaultValue_ = String(val);
        return this;
    }

}