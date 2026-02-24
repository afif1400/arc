pub mod ast;
pub mod lexer;
pub mod parser;
pub mod validator;
pub mod layout;
pub mod svg;
pub mod themes;
pub mod fmt;
pub mod mcp;

/// Convenience: parse, validate, layout, and render in one call.
pub fn render(source: &str, theme_name: Option<&str>) -> RenderOutput {
    let parse_result = parser::parse(source);
    let (validation, resolved) = validator::validate(&parse_result.document, &parse_result.diagnostics);
    let layout_result = layout::compute_layout(&resolved);
    let theme = themes::get_theme(theme_name.unwrap_or(resolved.theme_name()));
    let svg_output = svg::render_svg(&layout_result, &theme);

    RenderOutput {
        svg: svg_output,
        width: layout_result.width,
        height: layout_result.height,
        diagnostics: validation.errors,
        valid: validation.valid,
    }
}

pub struct RenderOutput {
    pub svg: String,
    pub width: f64,
    pub height: f64,
    pub diagnostics: Vec<ast::Diagnostic>,
    pub valid: bool,
}
