use crate::core::context::{append_custom_link, update_post_creation};
use crate::core::nodes::{GeometryNodeRepeatInput, GeometryNodeRepeatOutput};
use crate::core::types::{Int, NodeSocket, SocketDef};
use std::fmt::Write;

/// manually link
fn add_custom_link<T>(src: &NodeSocket<T>, dst_node: &str, index: usize) {
    if src.is_literal {
        let script = format!(
            "{}.inputs[{}].default_value = {}\n",
            dst_node, index, src.python_expr
        );
        append_custom_link(dst_node, script);
    } else {
        let script = format!(
            "tree.links.new({}, {}.inputs[{}])\n",
            src.python_expr, dst_node, index
        );
        append_custom_link(dst_node, script);
    }
}

pub trait RepeatItems {
    fn setup_items(out_name: &str, post_code: &mut String)
    where
        Self: Sized;
    fn link_initial(&self, in_name: &str);
    fn create_inner(in_name: &str) -> Self
    where
        Self: Sized;
    fn link_result(&self, out_name: &str);
    fn create_output(out_name: &str) -> Self
    where
        Self: Sized;
}

// for empty tuple ==================================================
impl RepeatItems for () {
    fn setup_items(_out_name: &str, _post_code: &mut String) {}
    fn link_initial(&self, _in_name: &str) {}
    fn create_inner(_in_name: &str) -> Self {}
    fn link_result(&self, _out_name: &str) {}
    fn create_output(_out_name: &str) -> Self {}
}

// for at least one element tuple ===================================
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
                $( add_custom_link(&self.$idx, in_name, $idx + 1); )+
            }
            fn create_inner(in_name: &str) -> Self {
                ( $( NodeSocket::<$T>::new_output(format!("{}.outputs[{}]", in_name, $idx + 1)), )+ )
            }
            fn link_result(&self, out_name: &str) {
                $( add_custom_link(&self.$idx, out_name, $idx); )+
            }
            fn create_output(out_name: &str) -> Self {
                ( $( NodeSocket::<$T>::new_output(format!("{}.outputs[{}]", out_name, $idx)), )+ )
            }
        }
    };
}

// RepeatItems is implemented for tuples of NodeSocket up to arity 6.
// To support higher arities, add further impl_repeat_items! invocations.
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
    let rep_in = GeometryNodeRepeatInput::new().with_iterations(iterations);

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
    fn test_repeat_zone_empty_tuple() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();
        context::enter_zone();

        let () = repeat_zone(5, (), |()| ());

        let nodes = context::exit_zone();
        let in_node = nodes
            .iter()
            .find(|n| n.bl_idname == "GeometryNodeRepeatInput")
            .unwrap();

        assert!(in_node.post_creation_script.contains("pair_with_output"));
        assert!(!in_node.post_creation_script.contains("repeat_items.new"));
        assert!(in_node.custom_links_script.is_empty());
    }

    #[test]
    fn test_repeat_zone_single_element() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();
        context::enter_zone();

        let initial_geo = NodeSocket::<Geo>::new_output("source_geo_expr");

        let (out_geo,) = repeat_zone(5, (initial_geo,), |(geo,)| {
            assert!(
                geo.python_expr.contains(".outputs[1]"),
                "Inner socket must reference outputs[1] to skip 'Iteration' output"
            );
            (geo,)
        });

        let nodes = context::exit_zone();

        assert!(out_geo.python_expr.contains(".outputs[0]"));

        let mut found_setup = false;
        let mut in_node_name = String::new();

        for node in &nodes {
            let post_code = &node.post_creation_script;
            if post_code.contains("pair_with_output") {
                found_setup = true;
                in_node_name = node.name.clone();
                assert!(post_code.contains("repeat_items.clear()"));
                assert!(post_code.contains("repeat_items.new('GEOMETRY', 'Geometry')"));
                assert!(!post_code.contains("FLOAT"));
            }
        }
        assert!(found_setup);

        let in_node = nodes.iter().find(|n| n.name == in_node_name).unwrap();
        let expected_link = format!("{}.inputs[1]", in_node_name);
        assert!(
            in_node.custom_links_script.contains(&expected_link),
            "Initial item should be linked to inputs[1], not inputs[0]"
        );
    }

    #[test]
    fn test_repeat_zone_multi_elements() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();
        context::enter_zone();

        let initial_geo = NodeSocket::<Geo>::new_output("source_geo");
        let initial_float = NodeSocket::<Float>::new_output("source_float");
        let initial_vec = NodeSocket::<Vector>::new_output("source_vec");

        let (out_g, out_f, out_v) = repeat_zone(
            10,
            (initial_geo, initial_float, initial_vec),
            |(g, f, v)| {
                assert!(g.python_expr.contains(".outputs[1]"));
                assert!(f.python_expr.contains(".outputs[2]"));
                assert!(v.python_expr.contains(".outputs[3]"));

                let new_f = &f + 1.0;
                (g, new_f, v)
            },
        );

        let nodes = context::exit_zone();

        assert!(out_g.python_expr.contains(".outputs[0]"));
        assert!(out_f.python_expr.contains(".outputs[1]"));
        assert!(out_v.python_expr.contains(".outputs[2]"));

        let mut in_node_name = String::new();
        let mut out_node_name = String::new();

        for node in &nodes {
            if node.bl_idname == "GeometryNodeRepeatInput" {
                in_node_name = node.name.clone();
            } else if node.bl_idname == "GeometryNodeRepeatOutput" {
                out_node_name = node.name.clone();
            }

            let post_code = &node.post_creation_script;
            if post_code.contains("pair_with_output") {
                assert!(post_code.contains("repeat_items.new('GEOMETRY', 'Geometry')"));
                assert!(post_code.contains("repeat_items.new('FLOAT', 'Value')"));
                assert!(post_code.contains("repeat_items.new('VECTOR', 'Vector')"));
            }
        }

        let in_node = nodes.iter().find(|n| n.name == in_node_name).unwrap();
        let in_link_count = in_node
            .custom_links_script
            .matches("tree.links.new")
            .count();
        let out_node = nodes.iter().find(|n| n.name == out_node_name).unwrap();
        let out_link_count = out_node
            .custom_links_script
            .matches("tree.links.new")
            .count();
        assert_eq!(out_link_count, 3, "expected 3 result links on RepeatOutput");
        assert_eq!(in_link_count, 3, "expected 3 initial links on RepeatInput");
        assert!(
            in_node
                .custom_links_script
                .contains(&format!("{}.inputs[1]", in_node_name))
        );
        assert!(
            in_node
                .custom_links_script
                .contains(&format!("{}.inputs[2]", in_node_name))
        );
        assert!(
            in_node
                .custom_links_script
                .contains(&format!("{}.inputs[3]", in_node_name))
        );

        assert!(
            out_node
                .custom_links_script
                .contains(&format!("{}.inputs[0]", out_node_name))
        );
        assert!(
            out_node
                .custom_links_script
                .contains(&format!("{}.inputs[1]", out_node_name))
        );
        assert!(
            out_node
                .custom_links_script
                .contains(&format!("{}.inputs[2]", out_node_name))
        );
    }
}
