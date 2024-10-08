export function addImmutableGlobal(name: string, value: any): void {
    Object.defineProperty(globalThis, name, {
        value: value,
        writable: false,
        configurable: globalThis.SJS_REPL || false,
        enumerable: true,
    });
}