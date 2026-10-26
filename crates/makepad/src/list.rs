//! `mod.quinn.sort_by(items, field, descending)`: the list sort the lowered view
//! language calls for `rows.sort_by(...)` and `rows.sort_by_key(...)`.
//!
//! Makepad script compares with `<` by casting both sides to numbers, so text
//! cannot be ordered in script. The sort runs here: numbers by value, text
//! without regard to case, items missing the field last, equal keys in their
//! old order. The lowerer's `Sim` follows the same rule.

use std::cmp::Ordering;

use makepad_widgets::*;

enum Key {
    Number(f64),
    Text(String),
    Missing,
}

fn compare(a: &Key, b: &Key) -> Ordering {
    match (a, b) {
        (Key::Number(a), Key::Number(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
        (Key::Text(a), Key::Text(b)) => a.cmp(b),
        (Key::Number(_), Key::Text(_)) => Ordering::Less,
        (Key::Text(_), Key::Number(_)) => Ordering::Greater,
        (Key::Missing, Key::Missing) => Ordering::Equal,
        (Key::Missing, _) => Ordering::Greater,
        (_, Key::Missing) => Ordering::Less,
    }
}

pub fn script_mod(vm: &mut ScriptVm) {
    let quinn = vm.new_module(id!(quinn));
    vm.add_method(
        quinn,
        id_lut!(sort_by),
        script_args_def!(items = NIL, field = NIL, descending = NIL),
        |vm, args| {
            let items = script_value!(vm, args.items);
            let field = script_value!(vm, args.field);
            let descending = script_value!(vm, args.descending)
                .as_bool()
                .unwrap_or(false);
            let Some(array) = items.as_array() else {
                return items;
            };
            let Some(field) = vm.bx.heap.string_with(field, |_, text| text.to_string()) else {
                return items;
            };
            let field = LiveId::from_str(&field);
            let len = vm.bx.heap.array_len(array);
            let mut rows = Vec::with_capacity(len);
            for index in 0..len {
                let trap = vm.bx.threads.cur_ref().trap.pass();
                let item = vm.bx.heap.array_index(array, index, trap);
                let value = match item.as_object() {
                    Some(object) => {
                        let trap = vm.bx.threads.cur_ref().trap.pass();
                        vm.bx.heap.value(object, field.into(), trap)
                    }
                    None => NIL,
                };
                let key = if let Some(number) = value.as_number() {
                    Key::Number(number)
                } else if let Some(text) = vm.bx.heap.string_with(value, |_, text| text.to_lowercase()) {
                    Key::Text(text)
                } else {
                    Key::Missing
                };
                rows.push((key, item));
            }
            rows.sort_by(|a, b| {
                let order = compare(&a.0, &b.0);
                if descending {
                    order.reverse()
                } else {
                    order
                }
            });
            let out = vm.bx.heap.new_array();
            for (_, item) in rows {
                vm.bx.heap.array_push_unchecked(out, item);
            }
            out.into()
        },
    );
}
