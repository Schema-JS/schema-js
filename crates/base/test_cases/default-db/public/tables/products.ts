export default function main() {
    const { Table, Column } = SchemaJS;
    return new Table("products")
        .addColumn(new Column("id").string())
}