export function addImmutableGlobal(name: string, value: any): void {
    Object.defineProperty(globalThis, name, {
        value: value,
        writable: false,
        configurable: false,
        enumerable: true,
    });
}