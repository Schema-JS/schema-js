export enum HelperType {
    CustomQuery = "CustomQuery",
    InsertHook = "InsertHook"
}

export type HelperCbType = (...args: any[]) => any;

export class Helper {

    public identifier: string = "";
    public internalType: HelperType;
    public cb: HelperCbType;

    constructor(identifier: string, internalType: HelperType, cb: HelperCbType) {
        if(identifier) {
            this.identifier = identifier;
        }
        this.internalType = internalType;
        this.cb = cb;
    }

}