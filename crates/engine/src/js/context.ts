import { addImmutableGlobal } from "ext:sjs_core/src/js/fieldUtils.ts";
interface Params {
    dbName: string,
    tblName: string,
}
export const initializeDbContext = (params: Params) => {
    addImmutableGlobal("SJS_CONTEXT", {
        ...(globalThis.SJS_CONTEXT || {}),
        ...params
    });
}