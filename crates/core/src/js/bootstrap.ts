import { addImmutableGlobal } from "ext:sjs_core/src/js/fieldUtils.ts";
import { SJSGlobal } from "ext:sjs_core/src/js/global.ts";
import { initializeDbContext } from "ext:sjs_engine/src/js/context.ts";
import { use, exit, close } from "ext:sjs_repl/src/js/repl.ts";

interface BootstrapParams {
    repl: boolean
}

globalThis.bootstrap = (params: BootstrapParams) => {

    // We should delete this after initialization,
    // Deleting it during bootstrapping can backfire
    delete globalThis.__bootstrap;
    delete globalThis.bootstrap;

    addImmutableGlobal("SchemaJS", SJSGlobal.SchemaJS);

    if(params.repl) {
        addImmutableGlobal("SJS_REPL", true);
        addImmutableGlobal("use", use);
        addImmutableGlobal("exit", exit);
        addImmutableGlobal("close", close);
    }

    globalThis.initializeDbContext({
        tblName: undefined,
        dbName: undefined,
        REPL_EXIT: false
    })


    delete globalThis.bootstrap;
}

globalThis.initializeDbContext = (params) => {
    initializeDbContext(params);
    if(!globalThis.SJS_REPL) {
        delete globalThis.initializeDbContext;
    }
}