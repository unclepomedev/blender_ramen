use proc_macro::TokenStream;
use quote::quote;
use syn::fold::Fold;
use syn::{Expr, parse_macro_input};

/// A struct for traversing and rewriting the AST.
///
/// It transforms path expressions (like variables) by appending `.clone()` to them,
/// which is useful for reusing variables multiple times in mathematical expressions.
struct MathFolder;

impl Fold for MathFolder {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        let folded = syn::fold::fold_expr(self, expr);

        match &folded {
            // Unconditionally rewrite all path expressions to `path.clone()`.
            // This includes variables, constants, and enum variants.
            Expr::Path(path) => {
                if path.path.segments.len() == 1 {
                    let ident_str = path.path.segments[0].ident.to_string();
                    let math_funcs = ["sin", "cos", "tan", "pow", "round", "sqrt"];
                    if math_funcs.contains(&ident_str.as_str()) {
                        return folded;
                    }
                }
                return syn::parse_quote!( #path.clone() );
            }

            Expr::Call(call) => {
                if let Expr::Path(func_path) = &*call.func {
                    let func_name = func_path.path.segments.last().unwrap().ident.to_string();
                    let args = &call.args;

                    let blender_op = match func_name.as_str() {
                        "sin" => "SINE",
                        "cos" => "COSINE",
                        "tan" => "TANGENT",
                        "round" => "ROUND",
                        "sqrt" => "SQRT",
                        "pow" => "POWER",
                        _ => return folded,  // TODO: implement other functions
                    };

                    if args.len() == 1 {
                        let arg = &args[0];
                        return syn::parse_quote! {
                            crate::core::nodes::ShaderNodeMath::new()
                                .with_operation(#blender_op)
                                .set_input(0, #arg)
                                .out_value()
                        };
                    }
                    else if args.len() == 2 {
                        let arg1 = &args[0];
                        let arg2 = &args[1];
                        return syn::parse_quote! {
                            crate::core::nodes::ShaderNodeMath::new()
                                .with_operation(#blender_op)
                                .set_input(0, #arg1)
                                .set_input(1, #arg2)
                                .out_value()
                        };
                    }
                }
            }
            _ => {}
        }

        folded
    }
}

/// A macro for NodeSocket arithmetic expressions.
///
/// This macro simplifies writing arithmetic operations by automatically cloning path expressions.
///
/// ### Transformation Semantics
/// - All path expressions (e.g., `a`, `my_var`) in the input are rewritten to `path.clone()`.
/// - Literals (e.g., `2.0`, `10`) are left untouched.
///
/// ### Intended Use Case
/// Designed primarily for `NodeSocket` arithmetic where variables are often reused and need to be cloned.
///
/// ### Limitations
/// All path expressions are unconditionally cloned. This includes:
/// - Single-segment paths (variables)
/// - Multi-segment paths (constants like `MY_CONST`, enum variants like `Option::None`)
/// For `Copy` types or constants, this may result in unnecessary `.clone()` calls.
///
/// ### Example
/// ```ignore
/// ramen_math!( (a + b) * c / 2.0 )
/// // becomes: (a.clone() + b.clone()) * c.clone() / 2.0
/// ```
#[proc_macro]
pub fn ramen_math(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as Expr);

    let mut folder = MathFolder;
    let expanded = folder.fold_expr(expr);

    TokenStream::from(quote!( #expanded ))
}
