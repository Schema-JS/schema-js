import { addImmutableGlobal } from "ext:sjs_core/src/js/fieldUtils.ts";
import { SJSGlobal } from "ext:sjs_core/src/js/global.ts";
import { initializeDbContext } from "ext:sjs_engine/src/js/context.ts";

interface BootstrapParams {
    repl: boolean
}

globalThis.bootstrap = (params: BootstrapParams) => {

    // We should delete this after initialization,
    // Deleting it during bootstrapping can backfire
    delete globalThis.__bootstrap;
    delete globalThis.bootstrap;

    addImmutableGlobal("SchemeJS", SJSGlobal.SchemeJS);

    if(params.repl) {
        addImmutableGlobal("SJS_REPL", true);
    }

    delete globalThis.bootstrap;
}

globalThis.initializeDbContext = (params) => {
    initializeDbContext(params);
    if(!globalThis.SJS_REPL) {
        delete globalThis.initializeDbContext;
    }
}