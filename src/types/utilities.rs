use std::collections::HashSet;
use ast::InputValue;
use schema::model::{SchemaType, TypeType};
use schema::meta::{MetaType, InputObjectMeta, EnumMeta};

pub fn is_valid_literal_value(schema: &SchemaType, arg_type: &TypeType, arg_value: &InputValue) -> bool {
    match *arg_type {
        TypeType::NonNull(ref inner) => {
            if arg_value.is_null() {
                false
            }
            else {
                is_valid_literal_value(schema, inner, arg_value)
            }
        }
        TypeType::List(ref inner) => {
            match *arg_value {
                InputValue::List(ref items) => items.iter().all(|i| is_valid_literal_value(schema, inner, &i.item)),
                ref v => is_valid_literal_value(schema, inner, v),
            }
        }
        TypeType::Concrete(t) => {
            // Even though InputValue::String can be parsed into an enum, they
            // are not valid as enum *literals* in a GraphQL query.
            if let (&InputValue::String(_), Some(&MetaType::Enum(EnumMeta { .. })))
                = (arg_value, arg_type.to_concrete()) {
                return false;
            }

            match *arg_value {
                InputValue::Null |
                InputValue::Variable(_) => true,
                ref v @ InputValue::Int(_) |
                ref v @ InputValue::Float(_) |
                ref v @ InputValue::String(_) |
                ref v @ InputValue::Boolean(_) |
                ref v @ InputValue::Enum(_) => {
                    if let Some(parse_fn) = t.input_value_parse_fn() {
                        parse_fn(v)
                    } else {
                        false
                    }
                },
                InputValue::List(_) => false,
                InputValue::Object(ref obj) => {
                    if let MetaType::InputObject(InputObjectMeta { ref input_fields, .. }) = *t {
                        let mut remaining_required_fields = input_fields.iter()
                            .filter_map(|f| if f.arg_type.is_non_null() { Some(&f.name) } else { None })
                            .collect::<HashSet<_>>();

                        let all_types_ok = obj.iter().all(|&(ref key, ref value)| {
                            remaining_required_fields.remove(&key.item);
                            if let Some(ref arg_type) = input_fields.iter()
                                .filter(|f| f.name == key.item)
                                .map(|f| schema.make_type(&f.arg_type))
                                .next()
                            {
                                is_valid_literal_value(schema, arg_type, &value.item)
                            }
                            else {
                                false
                            }
                        });

                        all_types_ok && remaining_required_fields.is_empty()
                    }
                    else {
                        false
                    }
                }
            }
        }
    }
}
