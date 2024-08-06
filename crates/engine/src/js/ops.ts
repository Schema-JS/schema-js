const core = globalThis.Deno.core;
export const insertRow = async (dbName: string, tableName: string, data: any) => {
    return await core.ops.op_engine_insert_row(
        dbName,
        tableName,
        data
    );
}