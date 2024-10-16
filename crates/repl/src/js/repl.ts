function setContext(db: string, tbl: string) {
    globalThis.SJS_CONTEXT.dbName = db;
    globalThis.SJS_CONTEXT.tblName = tbl;
}

function handleSingleArgument(data: string) {
    const { dbName, tblName } = globalThis.SJS_CONTEXT;

    if (!dbName) {
        globalThis.SJS_CONTEXT.dbName = data;
    } else if (!tblName) {
        globalThis.SJS_CONTEXT.tblName = data;
    } else {
        const [db, tbl] = data.split('.').filter(item => item.trim() !== "");
        if (db && tbl) {
            setContext(db, tbl);
        } else {
            return { REPL_ERR: 'AlreadyInContext' };
        }
    }
}

export const use = (...args: string[]) => {
    if (args.length === 1) {
        const data = args[0];
        return handleSingleArgument(data);
    } else if (args.length === 2) {
        const [db, tbl] = args;
        setContext(db, tbl);
    } else {
        return { REPL_ERR: 'UnexpectedUseArgsLength' };
    }
}

export const exit = () => {
    const { dbName, tblName } = globalThis.SJS_CONTEXT;
    if(tblName) {
        globalThis.SJS_CONTEXT.tblName = undefined;
    } else if(dbName) {
        globalThis.SJS_CONTEXT.dbName = undefined;
    } else {
        return { REPL_ERR: 'AlreadyInGlobal' };
    }
}

export const close = () => {
    globalThis.SJS_CONTEXT.REPL_EXIT = true;
}