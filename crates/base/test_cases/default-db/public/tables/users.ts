export default function main() {
    const { Table, Column, print, QueryBuilder, query } = SchemeJS;
    return new Table("users")
        .addColumn(new Column("id").string())
        .addColumn(new Column("username").string())
        .addColumn(new Column("password").string())
        .addColumn(new Column("enabled").boolean().withDefaultValue(true))
        .addQuery("searchRowLuis", async (req) => {
            let q = new QueryBuilder().and((and) => and.where("username", "=", "Luis"));
            let a = await query(q);
            print(JSON.stringify(a));
            return a;
        })
        .addQuery("helloWorld", (req) => { print(JSON.stringify(req)); })
}