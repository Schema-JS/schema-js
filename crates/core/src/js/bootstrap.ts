import { addImmutableGlobal } from "ext:sjs_core/src/js/fieldUtils.ts";
import { SJSGlobal } from "ext:sjs_core/src/js/global.ts";

globalThis.bootstrap = () => {

    // We should delete this after initialization,
    // Deleting it during bootstrapping can backfire
    delete globalThis.__bootstrap;
    delete globalThis.bootstrap;

    addImmutableGlobal("SchemeJS", SJSGlobal.SchemeJS);

    delete globalThis.bootstrap;
}
