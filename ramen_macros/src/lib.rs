use proc_macro::TokenStream;
use quote::quote;
use syn::fold::Fold;
use syn::{Expr, parse_macro_input};

/// Maps a Rust identifier to a Blender `ShaderNodeMath` operation name and the expected number of arguments.
fn get_blender_math_op(name: &str) -> Option<(&'static str, usize)> {
    match name {
        "sin" => Some(("SINE", 1)),
        "cos" => Some(("COSINE", 1)),
        "tan" => Some(("TANGENT", 1)),
        "asin" => Some(("ARCSINE", 1)),
        "acos" => Some(("ARCCOSINE", 1)),
        "atan" => Some(("ARCTANGENT", 1)),
        "sinh" => Some(("SINH", 1)),
        "cosh" => Some(("COSH", 1)),
        "tanh" => Some(("TANH", 1)),
        "sqrt" => Some(("SQRT", 1)),
        "exp" => Some(("EXPONENT", 1)),
        "round" => Some(("ROUND", 1)),
        "floor" => Some(("FLOOR", 1)),
        "ceil" => Some(("CEIL", 1)),
        "trunc" => Some(("TRUNC", 1)),
        "fract" => Some(("FRACT", 1)),
        "abs" => Some(("ABSOLUTE", 1)),
        "sign" => Some(("SIGN", 1)),
        "radians" => Some(("RADIANS", 1)),
        "degrees" => Some(("DEGREES", 1)),

        "log" => Some(("LOGARITHM", 2)),
        "atan2" => Some(("ARCTAN2", 2)),
        "pow" => Some(("POWER", 2)),
        "modulo" => Some(("MODULO", 2)),
        "min" => Some(("MINIMUM", 2)),
        "max" => Some(("MAXIMUM", 2)),
        "snap" => Some(("SNAP", 2)),
        "pingpong" => Some(("PINGPONG", 2)),

        "wrap" => Some(("WRAP", 3)),
        "smooth_min" => Some(("SMOOTH_MIN", 3)),
        "smooth_max" => Some(("SMOOTH_MAX", 3)),
        "compare" => Some(("COMPARE", 3)),
        "multiply_add" => Some(("MULTIPLY_ADD", 3)),

        _ => None,
    }
}

/// A structure for traversing the Abstract Syntax Tree (AST) and converting it into Blender node operations.
///
/// Main roles:
/// 1. Appends `.clone()` to path expressions (variables, etc.) to facilitate reuse within expressions.
/// 2. Replaces specific math function calls with code that generates `ShaderNodeMath` nodes.
struct MathFolder;

impl MathFolder {
    fn process_path(&mut self, path: &syn::ExprPath, folded: &Expr) -> Option<Expr> {
        // Do not clone identifiers registered as function names
        if path.path.segments.len() == 1 {
            let ident_str = path.path.segments[0].ident.to_string();
            if get_blender_math_op(&ident_str).is_some() {
                return Some(folded.clone());
            }
        }
        // Unconditionally append .clone() to other path expressions (variables, constants, etc.)
        // This avoids ownership issues with NodeSocket.
        Some(syn::parse_quote!( #path.clone() ))
    }

    fn process_call(&mut self, call: &syn::ExprCall, folded: &Expr) -> Option<Expr> {
        // Convert function calls to Blender ShaderNodeMath nodes
        if let Expr::Path(func_path) = &*call.func {
            let func_name = match func_path.path.segments.last() {
                Some(seg) => seg.ident.to_string(),
                None => return Some(folded.clone()),
            };

            let (blender_op, expected_args) = get_blender_math_op(&func_name)?;

            if call.args.len() != expected_args {
                let msg = format!(
                    "ramen_math!: function '{}' expects {} argument(s), but got {}",
                    func_name,
                    expected_args,
                    call.args.len()
                );
                return Some(syn::parse_quote! { compile_error!(#msg) });
            }

            let input_setters = call.args.iter().enumerate().map(|(i, arg)| {
                quote! { .set_input(#i, blender_ramen::core::types::NodeSocket::<blender_ramen::core::types::Float>::from(#arg)) }
            });

            return Some(syn::parse_quote! {
                blender_ramen::core::nodes::ShaderNodeMath::new()
                    .with_operation(#blender_op)
                    #(#input_setters)*
                    .out_value()
            });
        }
        None
    }

    fn process_unary(&mut self, un: &syn::ExprUnary) -> Option<Expr> {
        if let syn::UnOp::Not(_) = un.op {
            let inner = &un.expr;
            return Some(syn::parse_quote! {
                blender_ramen::core::nodes::FunctionNodeBooleanMath::new()
                    .with_operation("NOT")
                    .set_input(0, blender_ramen::core::types::NodeSocket::<blender_ramen::core::types::Bool>::from(#inner))
                    .out_boolean()
            });
        }
        None
    }

    fn process_binary(&mut self, bin: &syn::ExprBinary) -> Option<Expr> {
        let cmp_op = match bin.op {
            syn::BinOp::Eq(_) => Some("EQUAL"),
            syn::BinOp::Ne(_) => Some("NOT_EQUAL"),
            syn::BinOp::Lt(_) => Some("LESS_THAN"),
            syn::BinOp::Le(_) => Some("LESS_EQUAL"),
            syn::BinOp::Gt(_) => Some("GREATER_THAN"),
            syn::BinOp::Ge(_) => Some("GREATER_EQUAL"),
            _ => None,
        };

        if let Some(blender_op) = cmp_op {
            let left = &bin.left;
            let right = &bin.right;

            return Some(syn::parse_quote! {
                blender_ramen::core::nodes::FunctionNodeCompare::new()
                    .with_data_type("FLOAT")
                    .with_operation(#blender_op)
                    .set_input(0, blender_ramen::core::types::NodeSocket::<blender_ramen::core::types::Float>::from(#left))
                    .set_input(1, blender_ramen::core::types::NodeSocket::<blender_ramen::core::types::Float>::from(#right))
                    .out_result()
            });
        }

        let bool_op = match bin.op {
            syn::BinOp::And(_) | syn::BinOp::BitAnd(_) => Some("AND"),
            syn::BinOp::Or(_) | syn::BinOp::BitOr(_) => Some("OR"),
            syn::BinOp::BitXor(_) => Some("XOR"),
            _ => None,
        };

        if let Some(blender_op) = bool_op {
            let left = &bin.left;
            let right = &bin.right;

            return Some(syn::parse_quote! {
                blender_ramen::core::nodes::FunctionNodeBooleanMath::new()
                    .with_operation(#blender_op)
                    .set_input(0, blender_ramen::core::types::NodeSocket::<blender_ramen::core::types::Bool>::from(#left))
                    .set_input(1, blender_ramen::core::types::NodeSocket::<blender_ramen::core::types::Bool>::from(#right))
                    .out_boolean()
            });
        }

        None
    }
}

impl Fold for MathFolder {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        let folded = syn::fold::fold_expr(self, expr);

        match &folded {
            Expr::Path(path) => {
                if let Some(expr) = self.process_path(path, &folded) {
                    return expr;
                }
            }
            Expr::Call(call) => {
                if let Some(expr) = self.process_call(call, &folded) {
                    return expr;
                }
            }
            Expr::Unary(un) => {
                if let Some(expr) = self.process_unary(un) {
                    return expr;
                }
            }
            Expr::Binary(bin) => {
                if let Some(expr) = self.process_binary(bin) {
                    return expr;
                }
            }
            _ => {}
        }

        folded
    }
}

/// A macro for describing arithmetic expressions for NodeSocket.
///
/// Generates code that builds a Blender `ShaderNodeMath` node tree using standard Rust
/// arithmetic symbols (`+`, `-`, `*`, `/`) and math functions.
///
/// ### Transformation Mechanism
/// 1. **Automatic Variable Cloning**: Path expressions (variables or constants) in the expression
///    are automatically appended with `.clone()`. This allows the same variable to be reused multiple times.
/// 2. **Function Call Conversion**: Supported function calls are converted into corresponding `ShaderNodeMath` operations.
/// 3. **Literals**: Numeric literals (e.g., `2.0`) are preserved as is.
///
/// ### Supported Functions
/// Supports the following functions available in `ShaderNodeMath` for Blender 5.x and later:
///
/// - **1 argument**: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `sqrt`, `exp`, `round`, `floor`, `ceil`, `trunc`, `fract`, `abs`, `sign`, `radians`, `degrees`
/// - **2 arguments**: `log`, `atan2`, `pow`, `modulo`, `min`, `max`, `snap`, `pingpong`
/// - **3 arguments**: `wrap`, `smooth_min`, `smooth_max`, `compare`, `multiply_add`
///
/// ### Example
/// ```ignore
/// let a = NodeSocket::<Float>::from(10.0);
/// let b = NodeSocket::<Float>::from(5.0);
/// let result = ramen_math!( sin(a + b) * 2.0 );
/// ```
///
/// ### Limitations
/// - Path expressions are appended with `.clone()` unless the single-segment name matches a
///   supported math function. This means:
///   - **Unnecessary clones** may occur for `Copy` constants or enum variants.
///   - **Missing clones** (and potential move errors) may occur if a variable is named
///     identically to a supported function (e.g., naming a variable `sin` or `cos`).
///     Avoid reusing supported function names as variable names inside `ramen_math!`.
#[proc_macro]
pub fn ramen_math(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as Expr);
    let mut folder = MathFolder;
    let expanded = folder.fold_expr(expr);
    TokenStream::from(quote!( #expanded ))
}
