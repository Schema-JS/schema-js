export default function main() {
    const { Table, Column } = globalThis.SchemeJS;
    return new Table("users")
        .addColumn(new Column("id").string())
        .addColumn(new Column("username").string())
        .addColumn(new Column("password").string())
        .addColumn(new Column("enabled").boolean().withDefaultValue(true))
}