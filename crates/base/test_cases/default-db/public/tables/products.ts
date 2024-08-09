export default function main() {
    const { Table, Column } = SchemeJS;
    return new Table("products")
        .addColumn(new Column("id").string())
}