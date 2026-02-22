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

        if let Expr::Path(path) = &folded {
            // Unconditionally rewrite all path expressions to `path.clone()`.
            // This includes variables, constants, and enum variants.
            // e.g.: `a` -> `a.clone()`
            return syn::parse_quote!( #path.clone() );
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
