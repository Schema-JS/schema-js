pub fn to_prost_struct(
    json: serde_json::Map<String, serde_json::Value>,
) -> Result<prost_types::Struct, ()> {
    let fields: Result<_, ()> = json
        .into_iter()
        .map(|(k, v)| serde_json_to_prost(v).map(|v| (k, v)))
        .collect();

    fields.map(|fields| prost_types::Struct { fields })
}

pub fn serde_json_to_prost(json: serde_json::Value) -> Result<prost_types::Value, ()> {
    use prost_types::value::Kind::*;
    use serde_json::Value::*;

    let kind = match json {
        Null => Ok(NullValue(0)),
        Bool(v) => Ok(BoolValue(v)),
        Number(n) => n.as_f64().map(NumberValue).ok_or(()), // Return an error if the number can't be represented as f64
        String(s) => Ok(StringValue(s)),
        Array(v) => {
            let values: Result<_, ()> = v.into_iter().map(serde_json_to_prost).collect();
            values.map(|v| ListValue(prost_types::ListValue { values: v }))
        }
        Object(v) => to_prost_struct(v).map(StructValue),
    };

    kind.map(|k| prost_types::Value { kind: Some(k) })
}

pub fn prost_to_serde_json(x: prost_types::Value) -> Result<serde_json::Value, ()> {
    use prost_types::value::Kind::*;
    use serde_json::Value::*;

    match x.kind {
        Some(x) => match x {
            NullValue(_) => Ok(Null),
            BoolValue(v) => Ok(Bool(v)),
            NumberValue(n) => serde_json::Number::from_f64(n).map(Number).ok_or(()), // Return an error if `from_f64` returns None
            StringValue(s) => Ok(String(s)),
            ListValue(lst) => {
                let values: Result<_, ()> =
                    lst.values.into_iter().map(prost_to_serde_json).collect();
                values.map(Array)
            }
            StructValue(v) => {
                let fields: Result<_, ()> = v
                    .fields
                    .into_iter()
                    .map(|(k, v)| prost_to_serde_json(v).map(|v| (k, v)))
                    .collect();
                fields.map(Object)
            }
        },
        None => Ok(Null),
    }
}
