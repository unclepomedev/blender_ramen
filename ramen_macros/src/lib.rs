use proc_macro::TokenStream;
use quote::quote;
use syn::fold::Fold;
use syn::{Expr, parse_macro_input};

/// A struct for traversing and rewriting the AST
struct MathFolder;

impl Fold for MathFolder {
    fn fold_expr(&mut self, expr: Expr) -> Expr {
        let folded = syn::fold::fold_expr(self, expr);

        if let Expr::Path(path) = &folded {
            // e.g.: `a` -> `a.clone()`
            return syn::parse_quote!( #path.clone() );
        }

        folded
    }
}

/// math macro
/// e.g.: ramen_math!( (a + b) * c / 2.0 )
#[proc_macro]
pub fn ramen_math(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as Expr);

    let mut folder = MathFolder;
    let expanded = folder.fold_expr(expr);

    TokenStream::from(quote!( #expanded ))
}
