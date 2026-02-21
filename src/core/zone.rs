use crate::core::context::{append_custom_link, update_post_creation};
use crate::core::nodes::{GeometryNodeRepeatInput, GeometryNodeRepeatOutput};
use crate::core::types::{Int, NodeSocket, SocketDef};
use std::fmt::Write;

/// manually link
fn add_custom_link(src_expr: &str, dst_node: &str, index: usize) {
    let script = format!(
        "tree.links.new({}, {}.inputs[{}])\n",
        src_expr, dst_node, index
    );
    append_custom_link(dst_node, script);
}

pub trait RepeatItems {
    fn setup_items(out_name: &str, post_code: &mut String);
    fn link_initial(&self, in_name: &str);
    fn create_inner(in_name: &str) -> Self;
    fn link_result(&self, out_name: &str);
    fn create_output(out_name: &str) -> Self;
}

// auto-generate RepeatItems
macro_rules! impl_repeat_items {
    ( $($idx:tt => $T:ident),+ ) => {
        impl<$($T: SocketDef),+> RepeatItems for ($(NodeSocket<$T>,)+) {
            fn setup_items(out_name: &str, post_code: &mut String) {
                $(
                    let _ = writeln!(
                        post_code,
                        "{}.repeat_items.new('{}', '{}')",
                        out_name, $T::socket_type(), $T::default_name()
                    );
                )+
            }
            fn link_initial(&self, in_name: &str) {
                $( add_custom_link(&self.$idx.python_expr, in_name, $idx); )+
            }
            fn create_inner(in_name: &str) -> Self {
                ( $( NodeSocket::<$T>::new_expr(format!("{}.outputs[{}]", in_name, $idx)), )+ )
            }
            fn link_result(&self, out_name: &str) {
                $( add_custom_link(&self.$idx.python_expr, out_name, $idx); )+
            }
            fn create_output(out_name: &str) -> Self {
                ( $( NodeSocket::<$T>::new_expr(format!("{}.outputs[{}]", out_name, $idx)), )+ )
            }
        }
    };
}

impl_repeat_items!(0 => T0);
impl_repeat_items!(0 => T0, 1 => T1);
impl_repeat_items!(0 => T0, 1 => T1, 2 => T2);
impl_repeat_items!(0 => T0, 1 => T1, 2 => T2, 3 => T3);
impl_repeat_items!(0 => T0, 1 => T1, 2 => T2, 3 => T3, 4 => T4);
impl_repeat_items!(0 => T0, 1 => T1, 2 => T2, 3 => T3, 4 => T4, 5 => T5);

/// build Repeat Zone of Geometry Nodes
pub fn repeat_zone<T, F>(iterations: impl Into<NodeSocket<Int>>, initial_items: T, body: F) -> T
where
    T: RepeatItems,
    F: FnOnce(T) -> T,
{
    let rep_out = GeometryNodeRepeatOutput::new();
    let rep_in = GeometryNodeRepeatInput::new().iterations(iterations);

    let in_name = &rep_in.name;
    let out_name = &rep_out.name;

    // auto-generate pairings and sockets
    let mut post_code = String::new();
    let _ = writeln!(&mut post_code, "{in_name}.pair_with_output({out_name})");
    let _ = writeln!(&mut post_code, "{out_name}.repeat_items.clear()");
    T::setup_items(out_name, &mut post_code);
    update_post_creation(in_name, post_code);

    initial_items.link_initial(in_name);

    let inner_items = T::create_inner(in_name);
    let res_items = body(inner_items);

    res_items.link_result(out_name);

    T::create_output(out_name)
}

// ----------------------------------------------------------------------------
// unittest
// ----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context;
    use crate::core::context::test_utils::GLOBAL_TEST_LOCK;
    use crate::core::types::{Float, Geo, Vector};

    #[test]
    fn test_repeat_zone_single_element() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();
        context::enter_zone();

        let initial_geo = NodeSocket::<Geo>::new_expr("source_geo_expr");
        let (out_geo,) = repeat_zone(5, (initial_geo,), |(geo,)| (geo,));
        let nodes = context::exit_zone();

        assert!(out_geo.python_expr.contains(".outputs[0]"));

        let mut found_setup = false;
        for node in &nodes {
            let post_code = &node.post_creation_script;
            if post_code.contains("pair_with_output") {
                found_setup = true;
                assert!(post_code.contains("repeat_items.clear()"));
                assert!(post_code.contains("repeat_items.new('GEOMETRY', 'Geometry')"));
                assert!(!post_code.contains("FLOAT"));
            }
        }
        assert!(
            found_setup,
            "Repeat Input node should have the setup post_creation_script"
        );
    }

    #[test]
    fn test_repeat_zone_multi_elements() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();
        context::enter_zone();

        let initial_geo = NodeSocket::<Geo>::new_expr("source_geo");
        let initial_float = NodeSocket::<Float>::new_expr("source_float");
        let initial_vec = NodeSocket::<Vector>::new_expr("source_vec");

        let (out_g, out_f, out_v) = repeat_zone(
            10,
            (initial_geo, initial_float, initial_vec),
            |(g, f, v)| {
                let new_f = &f + 1.0;
                (g, new_f, v)
            },
        );

        let nodes = context::exit_zone();

        assert!(out_g.python_expr.contains(".outputs[0]"));
        assert!(out_f.python_expr.contains(".outputs[1]"));
        assert!(out_v.python_expr.contains(".outputs[2]"));

        let mut found_setup = false;
        let mut link_count = 0;

        for node in &nodes {
            let post_code = &node.post_creation_script;
            if post_code.contains("pair_with_output") {
                found_setup = true;
                assert!(post_code.contains("repeat_items.new('GEOMETRY', 'Geometry')"));
                assert!(post_code.contains("repeat_items.new('FLOAT', 'Value')"));
                assert!(post_code.contains("repeat_items.new('VECTOR', 'Vector')"));
            }

            link_count += node.custom_links_script.matches("tree.links.new").count();
        }

        assert!(found_setup);

        assert_eq!(
            link_count, 6,
            "Should generate exactly 6 custom links for 3 items (in and out)"
        );
    }
}
