export default function main() {
    const { Table, Column } = SchemeJS;
    return new Table("users")
        .addColumn(new Column("id").string())
        .addColumn(new Column("username").string())
        .addColumn(new Column("password").string())
        .addColumn(new Column("enabled").boolean().withDefaultValue(true))
        .addQuery("helloWorld", () => { let x = "hello"; })
}